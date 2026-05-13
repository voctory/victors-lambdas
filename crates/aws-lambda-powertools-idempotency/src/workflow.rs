//! Idempotent handler workflow.

use std::time::SystemTime;

use serde::{Serialize, de::DeserializeOwned};

use crate::{
    IdempotencyConfig, IdempotencyError, IdempotencyExecutionError, IdempotencyKey,
    IdempotencyRecord, IdempotencyStatus, IdempotencyStore, hash_payload, key_from_payload,
};

/// Result of an idempotent handler invocation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IdempotencyOutcome<T> {
    /// The wrapped handler executed and produced this value.
    Executed(T),
    /// A previously stored response was replayed.
    Replayed(T),
}

impl<T> IdempotencyOutcome<T> {
    /// Returns whether the wrapped handler executed.
    #[must_use]
    pub const fn is_executed(&self) -> bool {
        matches!(self, Self::Executed(_))
    }

    /// Returns whether a stored response was replayed.
    #[must_use]
    pub const fn is_replayed(&self) -> bool {
        matches!(self, Self::Replayed(_))
    }

    /// Returns a reference to the invocation value.
    #[must_use]
    pub const fn value(&self) -> &T {
        match self {
            Self::Executed(value) | Self::Replayed(value) => value,
        }
    }

    /// Consumes the outcome and returns the invocation value.
    #[must_use]
    pub fn into_inner(self) -> T {
        match self {
            Self::Executed(value) | Self::Replayed(value) => value,
        }
    }
}

/// Coordinates idempotent handler execution with an idempotency store.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Idempotency<S> {
    config: IdempotencyConfig,
    store: S,
}

impl<S> Idempotency<S> {
    /// Creates an idempotency workflow with default configuration.
    #[must_use]
    pub fn new(store: S) -> Self {
        Self::with_config(store, IdempotencyConfig::default())
    }

    /// Creates an idempotency workflow with explicit configuration.
    #[must_use]
    pub const fn with_config(store: S, config: IdempotencyConfig) -> Self {
        Self { config, store }
    }

    /// Returns the idempotency configuration.
    #[must_use]
    pub const fn config(&self) -> &IdempotencyConfig {
        &self.config
    }

    /// Returns the backing idempotency store.
    #[must_use]
    pub const fn store(&self) -> &S {
        &self.store
    }

    /// Returns a mutable reference to the backing idempotency store.
    pub const fn store_mut(&mut self) -> &mut S {
        &mut self.store
    }

    /// Consumes the workflow and returns the backing idempotency store.
    #[must_use]
    pub fn into_store(self) -> S {
        self.store
    }
}

impl<S> Idempotency<S>
where
    S: IdempotencyStore,
{
    /// Executes a handler using a key derived from the hashed payload.
    ///
    /// Successful handler responses are JSON serialized and cached. Later calls
    /// with the same key replay the cached response without running the handler.
    ///
    /// # Errors
    ///
    /// Returns an idempotency error when key generation, store access, response
    /// serialization, or replay deserialization fails. Returns a handler error
    /// when the wrapped handler fails.
    pub fn execute_json<T, R, E>(
        &mut self,
        payload: &T,
        handler: impl FnOnce() -> Result<R, E>,
    ) -> Result<IdempotencyOutcome<R>, IdempotencyExecutionError<E>>
    where
        T: Serialize + ?Sized,
        R: Serialize + DeserializeOwned,
    {
        if self.config.disabled() {
            return handler()
                .map(IdempotencyOutcome::Executed)
                .map_err(IdempotencyExecutionError::Handler);
        }

        let key = key_from_payload(payload)?;
        self.execute_json_with_key(key, payload, handler)
    }

    /// Executes a handler using an explicit idempotency key and payload hash.
    ///
    /// Use this when the idempotency key is extracted from a stable subset of
    /// the payload while the full payload hash should still be validated.
    ///
    /// # Errors
    ///
    /// Returns an idempotency error when the key is empty, store access,
    /// response serialization, or replay deserialization fails. Returns a
    /// handler error when the wrapped handler fails.
    pub fn execute_json_with_key<T, R, E>(
        &mut self,
        key: impl Into<IdempotencyKey>,
        payload: &T,
        handler: impl FnOnce() -> Result<R, E>,
    ) -> Result<IdempotencyOutcome<R>, IdempotencyExecutionError<E>>
    where
        T: Serialize + ?Sized,
        R: Serialize + DeserializeOwned,
    {
        if self.config.disabled() {
            return handler()
                .map(IdempotencyOutcome::Executed)
                .map_err(IdempotencyExecutionError::Handler);
        }

        let key = self.scoped_key(key.into())?;
        let payload_hash = hash_payload(payload)?;
        let now = SystemTime::now();

        if let Some(record) = self.store.get(&key).map_err(IdempotencyError::from)? {
            match Self::evaluate_existing_record(&key, &payload_hash, &record, now)? {
                ExistingRecord::Replay(response) => {
                    return Ok(IdempotencyOutcome::Replayed(response));
                }
                ExistingRecord::Expired => {
                    self.store.remove(&key).map_err(IdempotencyError::from)?;
                }
            }
        }

        let in_progress_expires_at = now + self.config.in_progress_ttl();
        let in_progress = IdempotencyRecord::in_progress_until(key.clone(), in_progress_expires_at)
            .with_payload_hash(payload_hash.clone());
        self.store
            .put(in_progress)
            .map_err(IdempotencyError::from)?;

        match handler() {
            Ok(response) => {
                let response_data = serde_json::to_vec(&response)
                    .map_err(|error| IdempotencyError::serialization(error.to_string()))?;
                let completed_expires_at = now + self.config.record_ttl();
                let completed = IdempotencyRecord::completed_until(key, completed_expires_at)
                    .with_payload_hash(payload_hash)
                    .with_response_data(response_data);
                self.store.put(completed).map_err(IdempotencyError::from)?;
                Ok(IdempotencyOutcome::Executed(response))
            }
            Err(error) => {
                self.store.remove(&key).map_err(IdempotencyError::from)?;
                Err(IdempotencyExecutionError::Handler(error))
            }
        }
    }

    fn scoped_key(&self, key: IdempotencyKey) -> Result<IdempotencyKey, IdempotencyError> {
        if key.is_empty() {
            return Err(IdempotencyError::MissingKey);
        }

        match self.config.key_prefix() {
            Some(prefix) if !prefix.is_empty() => {
                Ok(IdempotencyKey::new(format!("{prefix}#{}", key.value())))
            }
            Some(_) | None => Ok(key),
        }
    }

    fn evaluate_existing_record<R>(
        key: &IdempotencyKey,
        payload_hash: &str,
        record: &IdempotencyRecord,
        now: SystemTime,
    ) -> Result<ExistingRecord<R>, IdempotencyError>
    where
        R: DeserializeOwned,
    {
        if let Some(stored_hash) = record.payload_hash() {
            if stored_hash != payload_hash {
                return Err(IdempotencyError::PayloadMismatch { key: key.clone() });
            }
        }

        match record.status_at(now) {
            IdempotencyStatus::Completed => {
                let response_data = record
                    .response_data()
                    .ok_or_else(|| IdempotencyError::MissingStoredResponse { key: key.clone() })?;
                let response = serde_json::from_slice(response_data)
                    .map_err(|error| IdempotencyError::serialization(error.to_string()))?;
                Ok(ExistingRecord::Replay(response))
            }
            IdempotencyStatus::InProgress => {
                Err(IdempotencyError::AlreadyInProgress { key: key.clone() })
            }
            IdempotencyStatus::Expired => Ok(ExistingRecord::Expired),
        }
    }
}

enum ExistingRecord<R> {
    Replay(R),
    Expired,
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use crate::{
        Idempotency, IdempotencyConfig, IdempotencyError, IdempotencyExecutionError,
        IdempotencyKey, IdempotencyOutcome, IdempotencyRecord, InMemoryIdempotencyStore,
        hash_payload,
    };

    #[test]
    fn execute_json_runs_once_and_replays_response() {
        let mut idempotency = Idempotency::new(InMemoryIdempotencyStore::new());
        let payload = json!({"order_id": "abc"});
        let mut calls = 0;

        let first = idempotency
            .execute_json(&payload, || {
                calls += 1;
                Ok::<_, &'static str>(json!({"status": "created"}))
            })
            .expect("first call succeeds");
        let second = idempotency
            .execute_json(&payload, || {
                calls += 1;
                Ok::<_, &'static str>(json!({"status": "duplicate"}))
            })
            .expect("second call replays");

        assert_eq!(calls, 1);
        assert_eq!(
            first,
            IdempotencyOutcome::Executed(json!({"status": "created"}))
        );
        assert_eq!(
            second,
            IdempotencyOutcome::Replayed(json!({"status": "created"}))
        );
    }

    #[test]
    fn execute_json_with_key_applies_configured_key_prefix() {
        let config = IdempotencyConfig::new(false).with_key_prefix("orders");
        let mut idempotency = Idempotency::with_config(InMemoryIdempotencyStore::new(), config);

        let outcome = idempotency
            .execute_json_with_key("order-1", &json!({"order_id": "1"}), || {
                Ok::<_, &'static str>(json!({"ok": true}))
            })
            .expect("handler succeeds");

        assert!(outcome.is_executed());
        assert!(
            idempotency
                .store()
                .contains(&IdempotencyKey::new("orders#order-1"))
        );
    }

    #[test]
    fn execute_json_removes_record_when_handler_fails() {
        let mut idempotency = Idempotency::new(InMemoryIdempotencyStore::new());
        let payload = json!({"order_id": "abc"});

        let result = idempotency.execute_json(&payload, || Err::<Value, _>("handler failed"));

        assert_eq!(
            result,
            Err(IdempotencyExecutionError::Handler("handler failed"))
        );
        assert!(idempotency.store().is_empty());
    }

    #[test]
    fn execute_json_rejects_existing_in_progress_record() {
        let payload = json!({"order_id": "abc"});
        let key = IdempotencyKey::new("order-1");
        let payload_hash = hash_payload(&payload).expect("payload hashes");
        let record = IdempotencyRecord::in_progress_until(
            key.clone(),
            std::time::SystemTime::now() + std::time::Duration::from_secs(60),
        )
        .with_payload_hash(payload_hash);
        let mut idempotency = Idempotency::new(InMemoryIdempotencyStore::new().with_record(record));

        let result = idempotency
            .execute_json_with_key(key.clone(), &payload, || Ok::<_, &'static str>(json!(null)));

        assert_eq!(
            result,
            Err(IdempotencyExecutionError::Idempotency(
                IdempotencyError::AlreadyInProgress { key }
            ))
        );
    }

    #[test]
    fn execute_json_rejects_payload_hash_mismatch() {
        let key = IdempotencyKey::new("order-1");
        let record = IdempotencyRecord::completed_until(
            key.clone(),
            std::time::SystemTime::now() + std::time::Duration::from_secs(60),
        )
        .with_payload_hash("old-hash")
        .with_response_data(br#"{"status":"created"}"#.to_vec());
        let mut idempotency = Idempotency::new(InMemoryIdempotencyStore::new().with_record(record));

        let result =
            idempotency.execute_json_with_key(key.clone(), &json!({"order_id": "abc"}), || {
                Ok::<_, &'static str>(json!({"status": "ignored"}))
            });

        assert_eq!(
            result,
            Err(IdempotencyExecutionError::Idempotency(
                IdempotencyError::PayloadMismatch { key }
            ))
        );
    }
}
