//! External cache-backed idempotency store.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::{
    AsyncIdempotencyStore, IdempotencyKey, IdempotencyRecord, IdempotencyStatus,
    IdempotencyStoreError, IdempotencyStoreFuture, IdempotencyStoreResult,
};

const DEFAULT_CACHE_KEY_PREFIX: &str = "idempotency#";

/// Asynchronous byte cache operations used by [`CacheIdempotencyStore`].
///
/// Implement this trait for Redis, Valkey, or another TTL-capable cache client.
/// Values are opaque bytes owned by the idempotency store.
pub trait AsyncIdempotencyCacheClient: Sync {
    /// Retrieves a raw cache value.
    ///
    /// # Errors
    ///
    /// Returns an error when the cache lookup fails.
    fn get<'a>(&'a self, key: &'a str) -> IdempotencyStoreFuture<'a, Option<Vec<u8>>>;

    /// Stores a raw cache value with a time-to-live.
    ///
    /// # Errors
    ///
    /// Returns an error when the cache write fails.
    fn set<'a>(
        &'a self,
        key: &'a str,
        value: Vec<u8>,
        ttl: Duration,
    ) -> IdempotencyStoreFuture<'a, ()>;

    /// Removes a raw cache value.
    ///
    /// # Errors
    ///
    /// Returns an error when the cache delete fails.
    fn remove<'a>(&'a self, key: &'a str) -> IdempotencyStoreFuture<'a, Option<Vec<u8>>>;
}

/// Idempotency store backed by an external TTL cache.
///
/// This adapter stores serialized idempotency records in any cache client that
/// implements [`AsyncIdempotencyCacheClient`]. It is intended for Redis,
/// Valkey, and similar cache services without forcing a concrete client crate
/// into the Powertools dependency graph.
#[derive(Clone, Debug)]
pub struct CacheIdempotencyStore<C> {
    client: C,
    key_prefix: String,
}

impl<C> CacheIdempotencyStore<C> {
    /// Creates a cache-backed idempotency store with the default key prefix.
    #[must_use]
    pub fn new(client: C) -> Self {
        Self::with_key_prefix(client, DEFAULT_CACHE_KEY_PREFIX)
    }

    /// Creates a cache-backed idempotency store with a custom key prefix.
    ///
    /// Blank prefixes are normalized to the default prefix.
    #[must_use]
    pub fn with_key_prefix(client: C, key_prefix: impl Into<String>) -> Self {
        let key_prefix = key_prefix.into();
        let key_prefix = key_prefix.trim();
        Self {
            client,
            key_prefix: if key_prefix.is_empty() {
                DEFAULT_CACHE_KEY_PREFIX.to_owned()
            } else {
                key_prefix.to_owned()
            },
        }
    }

    /// Returns the wrapped cache client.
    #[must_use]
    pub const fn client(&self) -> &C {
        &self.client
    }

    /// Consumes this store and returns the wrapped cache client.
    #[must_use]
    pub fn into_client(self) -> C {
        self.client
    }

    /// Returns the key prefix used for cache entries.
    #[must_use]
    pub fn key_prefix(&self) -> &str {
        &self.key_prefix
    }

    fn cache_key(&self, key: &IdempotencyKey) -> String {
        format!("{}{}", self.key_prefix, key.value())
    }
}

impl<C> AsyncIdempotencyStore for CacheIdempotencyStore<C>
where
    C: AsyncIdempotencyCacheClient,
{
    fn get<'a>(
        &'a self,
        key: &'a IdempotencyKey,
    ) -> IdempotencyStoreFuture<'a, Option<IdempotencyRecord>> {
        Box::pin(async move {
            let cache_key = self.cache_key(key);
            let Some(bytes) = self.client.get(&cache_key).await? else {
                return Ok(None);
            };

            let record = decode_record(&bytes)?;
            if record.is_expired_at(SystemTime::now()) {
                let _removed = self.client.remove(&cache_key).await?;
                Ok(None)
            } else {
                Ok(Some(record))
            }
        })
    }

    fn put(
        &self,
        record: IdempotencyRecord,
    ) -> IdempotencyStoreFuture<'_, Option<IdempotencyRecord>> {
        Box::pin(async move {
            let cache_key = self.cache_key(record.key());
            let old_record = self
                .client
                .get(&cache_key)
                .await?
                .map(|bytes| decode_record(&bytes))
                .transpose()?;

            let ttl = record
                .expires_at()
                .duration_since(SystemTime::now())
                .unwrap_or(Duration::ZERO);
            if ttl.is_zero() {
                let _removed = self.client.remove(&cache_key).await?;
                return Ok(old_record);
            }

            self.client
                .set(&cache_key, encode_record(&record)?, ttl)
                .await?;
            Ok(old_record)
        })
    }

    fn remove<'a>(
        &'a self,
        key: &'a IdempotencyKey,
    ) -> IdempotencyStoreFuture<'a, Option<IdempotencyRecord>> {
        Box::pin(async move {
            let cache_key = self.cache_key(key);
            self.client
                .remove(&cache_key)
                .await?
                .map(|bytes| decode_record(&bytes))
                .transpose()
        })
    }

    fn clear_expired(&self, _now: SystemTime) -> IdempotencyStoreFuture<'_, usize> {
        Box::pin(async { Ok(0) })
    }
}

#[derive(Deserialize, Serialize)]
struct CacheRecord {
    key: String,
    status: CacheRecordStatus,
    expires_at_unix_seconds: u64,
    expires_at_subsec_nanos: u32,
    payload_hash: Option<String>,
    response_data: Option<Vec<u8>>,
}

#[derive(Deserialize, Serialize)]
enum CacheRecordStatus {
    #[serde(rename = "INPROGRESS")]
    InProgress,
    #[serde(rename = "COMPLETED")]
    Completed,
}

fn encode_record(record: &IdempotencyRecord) -> IdempotencyStoreResult<Vec<u8>> {
    let status = match record.status() {
        IdempotencyStatus::InProgress => CacheRecordStatus::InProgress,
        IdempotencyStatus::Completed => CacheRecordStatus::Completed,
        IdempotencyStatus::Expired => {
            return Err(IdempotencyStoreError::new(
                "expired idempotency records cannot be stored in cache",
            ));
        }
    };

    let (expires_at_unix_seconds, expires_at_subsec_nanos) =
        system_time_to_unix_parts(record.expires_at());

    serde_json::to_vec(&CacheRecord {
        key: record.key().value().to_owned(),
        status,
        expires_at_unix_seconds,
        expires_at_subsec_nanos,
        payload_hash: record.payload_hash().map(ToOwned::to_owned),
        response_data: record.response_data().map(ToOwned::to_owned),
    })
    .map_err(|error| {
        IdempotencyStoreError::new(format!(
            "failed to serialize idempotency cache record: {error}"
        ))
    })
}

fn decode_record(bytes: &[u8]) -> IdempotencyStoreResult<IdempotencyRecord> {
    let record = serde_json::from_slice::<CacheRecord>(bytes).map_err(|error| {
        IdempotencyStoreError::new(format!(
            "failed to deserialize idempotency cache record: {error}"
        ))
    })?;

    let expires_at = unix_parts_to_system_time(
        record.expires_at_unix_seconds,
        record.expires_at_subsec_nanos,
    );
    let mut output = match record.status {
        CacheRecordStatus::InProgress => {
            IdempotencyRecord::in_progress_until(record.key, expires_at)
        }
        CacheRecordStatus::Completed => IdempotencyRecord::completed_until(record.key, expires_at),
    };

    if let Some(payload_hash) = record.payload_hash {
        output = output.with_payload_hash(payload_hash);
    }
    if let Some(response_data) = record.response_data {
        output = output.with_response_data(response_data);
    }

    Ok(output)
}

fn system_time_to_unix_parts(time: SystemTime) -> (u64, u32) {
    let duration = time.duration_since(UNIX_EPOCH).unwrap_or(Duration::ZERO);
    (duration.as_secs(), duration.subsec_nanos())
}

fn unix_parts_to_system_time(seconds: u64, subsec_nanos: u32) -> SystemTime {
    UNIX_EPOCH + Duration::new(seconds, subsec_nanos)
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeMap,
        sync::{
            Mutex,
            atomic::{AtomicUsize, Ordering},
        },
        time::{Duration, SystemTime},
    };

    use futures_executor::block_on;

    use super::{AsyncIdempotencyCacheClient, CacheIdempotencyStore};
    use crate::{AsyncIdempotencyStore, IdempotencyKey, IdempotencyRecord, IdempotencyStoreFuture};

    #[derive(Debug, Default)]
    struct FakeCacheClient {
        values: Mutex<BTreeMap<String, Vec<u8>>>,
        ttl: Mutex<BTreeMap<String, Duration>>,
        removes: AtomicUsize,
    }

    impl FakeCacheClient {
        fn contains_key(&self, key: &str) -> bool {
            self.values.lock().expect("values").contains_key(key)
        }

        fn ttl_for(&self, key: &str) -> Option<Duration> {
            self.ttl.lock().expect("ttl").get(key).copied()
        }

        fn remove_count(&self) -> usize {
            self.removes.load(Ordering::SeqCst)
        }
    }

    impl AsyncIdempotencyCacheClient for FakeCacheClient {
        fn get<'a>(&'a self, key: &'a str) -> IdempotencyStoreFuture<'a, Option<Vec<u8>>> {
            Box::pin(async move { Ok(self.values.lock().expect("values").get(key).cloned()) })
        }

        fn set<'a>(
            &'a self,
            key: &'a str,
            value: Vec<u8>,
            ttl: Duration,
        ) -> IdempotencyStoreFuture<'a, ()> {
            Box::pin(async move {
                self.values
                    .lock()
                    .expect("values")
                    .insert(key.to_owned(), value);
                self.ttl.lock().expect("ttl").insert(key.to_owned(), ttl);
                Ok(())
            })
        }

        fn remove<'a>(&'a self, key: &'a str) -> IdempotencyStoreFuture<'a, Option<Vec<u8>>> {
            Box::pin(async move {
                self.removes.fetch_add(1, Ordering::SeqCst);
                self.ttl.lock().expect("ttl").remove(key);
                Ok(self.values.lock().expect("values").remove(key))
            })
        }
    }

    #[test]
    fn cache_store_round_trips_records_with_ttl() {
        let client = FakeCacheClient::default();
        let store = CacheIdempotencyStore::new(client);
        let key = IdempotencyKey::from("request-1");
        let record = IdempotencyRecord::completed_until(
            key.clone(),
            SystemTime::now() + Duration::from_secs(60),
        )
        .with_payload_hash("payload-hash")
        .with_response_data(b"response".to_vec());

        let old_record = block_on(store.put(record.clone())).expect("put should succeed");
        let cached = block_on(store.get(&key)).expect("get should succeed");

        assert_eq!(old_record, None);
        assert_eq!(cached, Some(record));
        assert!(
            store
                .client()
                .ttl_for("idempotency#request-1")
                .expect("ttl")
                <= Duration::from_secs(60)
        );
    }

    #[test]
    fn cache_store_removes_expired_records_on_read() {
        let client = FakeCacheClient::default();
        let store = CacheIdempotencyStore::new(client);
        let key = IdempotencyKey::from("request-1");
        let record = IdempotencyRecord::in_progress_until(
            key.clone(),
            SystemTime::now() + Duration::from_millis(1),
        );

        block_on(store.put(record)).expect("put should succeed");
        std::thread::sleep(Duration::from_millis(5));

        assert_eq!(block_on(store.get(&key)).expect("get should succeed"), None);
        assert!(!store.client().contains_key("idempotency#request-1"));
        assert_eq!(store.client().remove_count(), 1);
    }

    #[test]
    fn cache_store_returns_previous_record_on_put_and_remove() {
        let client = FakeCacheClient::default();
        let store = CacheIdempotencyStore::with_key_prefix(client, "idem:");
        let key = IdempotencyKey::from("request-1");
        let first = IdempotencyRecord::in_progress_until(
            key.clone(),
            SystemTime::now() + Duration::from_secs(60),
        );
        let second = IdempotencyRecord::completed_until(
            key.clone(),
            SystemTime::now() + Duration::from_secs(120),
        );

        assert_eq!(block_on(store.put(first.clone())).expect("first put"), None);
        assert_eq!(
            block_on(store.put(second.clone())).expect("second put"),
            Some(first)
        );
        assert_eq!(block_on(store.remove(&key)).expect("remove"), Some(second));
        assert!(!store.client().contains_key("idem:request-1"));
    }

    #[test]
    fn blank_cache_prefix_uses_default_prefix() {
        let store = CacheIdempotencyStore::with_key_prefix(FakeCacheClient::default(), " ");

        assert_eq!(store.key_prefix(), "idempotency#");
    }

    #[test]
    fn cache_store_reports_invalid_cached_records() {
        let client = FakeCacheClient::default();
        client
            .values
            .lock()
            .expect("values")
            .insert("idempotency#request-1".to_owned(), b"not-json".to_vec());
        let store = CacheIdempotencyStore::new(client);
        let error = block_on(store.get(&IdempotencyKey::from("request-1")))
            .expect_err("invalid record should fail");

        assert!(
            error
                .message()
                .contains("failed to deserialize idempotency cache record")
        );
    }

    #[test]
    fn clear_expired_is_noop_for_ttl_cache_clients() {
        let store = CacheIdempotencyStore::new(FakeCacheClient::default());

        assert_eq!(
            block_on(store.clear_expired(SystemTime::now())).expect("clear should succeed"),
            0
        );
    }
}
