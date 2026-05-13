//! Idempotency persistence traits and in-memory store.

use std::{collections::BTreeMap, future::Future, pin::Pin, time::SystemTime};

use crate::{IdempotencyKey, IdempotencyRecord};

/// Result returned by idempotency store operations.
pub type IdempotencyStoreResult<T> = Result<T, IdempotencyStoreError>;

/// Boxed future returned by asynchronous idempotency stores.
pub type IdempotencyStoreFuture<'a, T> =
    Pin<Box<dyn Future<Output = IdempotencyStoreResult<T>> + Send + 'a>>;

/// Error returned by an idempotency persistence backend.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IdempotencyStoreError {
    message: String,
}

impl IdempotencyStoreError {
    /// Creates an idempotency store error.
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// Returns the error message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl std::fmt::Display for IdempotencyStoreError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for IdempotencyStoreError {}

/// Persistence operations for idempotency records.
pub trait IdempotencyStore {
    /// Retrieves a record by idempotency key.
    ///
    /// # Errors
    ///
    /// Returns an error when the backing store cannot complete the lookup.
    fn get(&self, key: &IdempotencyKey) -> IdempotencyStoreResult<Option<IdempotencyRecord>>;

    /// Inserts or replaces a record.
    ///
    /// # Errors
    ///
    /// Returns an error when the backing store cannot persist the record.
    fn put(
        &mut self,
        record: IdempotencyRecord,
    ) -> IdempotencyStoreResult<Option<IdempotencyRecord>>;

    /// Removes a record by idempotency key.
    ///
    /// # Errors
    ///
    /// Returns an error when the backing store cannot remove the record.
    fn remove(&mut self, key: &IdempotencyKey)
    -> IdempotencyStoreResult<Option<IdempotencyRecord>>;

    /// Removes expired records and returns the number removed.
    ///
    /// # Errors
    ///
    /// Returns an error when the backing store cannot remove expired records.
    fn clear_expired(&mut self, now: SystemTime) -> IdempotencyStoreResult<usize>;
}

/// Asynchronous persistence operations for idempotency records.
pub trait AsyncIdempotencyStore: Sync {
    /// Retrieves a record by idempotency key.
    ///
    /// # Errors
    ///
    /// Returns an error when the backing store cannot complete the lookup.
    fn get<'a>(
        &'a self,
        key: &'a IdempotencyKey,
    ) -> IdempotencyStoreFuture<'a, Option<IdempotencyRecord>>;

    /// Inserts or replaces a record.
    ///
    /// # Errors
    ///
    /// Returns an error when the backing store cannot persist the record.
    fn put(
        &self,
        record: IdempotencyRecord,
    ) -> IdempotencyStoreFuture<'_, Option<IdempotencyRecord>>;

    /// Removes a record by idempotency key.
    ///
    /// # Errors
    ///
    /// Returns an error when the backing store cannot remove the record.
    fn remove<'a>(
        &'a self,
        key: &'a IdempotencyKey,
    ) -> IdempotencyStoreFuture<'a, Option<IdempotencyRecord>>;

    /// Removes expired records and returns the number removed.
    ///
    /// # Errors
    ///
    /// Returns an error when the backing store cannot remove expired records.
    fn clear_expired(&self, now: SystemTime) -> IdempotencyStoreFuture<'_, usize>;
}

/// In-memory idempotency store for tests and local examples.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct InMemoryIdempotencyStore {
    records: BTreeMap<IdempotencyKey, IdempotencyRecord>,
}

impl InMemoryIdempotencyStore {
    /// Creates an empty in-memory idempotency store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates an in-memory idempotency store from records.
    #[must_use]
    pub fn from_records(records: impl IntoIterator<Item = IdempotencyRecord>) -> Self {
        records.into_iter().collect()
    }

    /// Inserts or replaces a record and returns the store.
    #[must_use]
    pub fn with_record(mut self, record: IdempotencyRecord) -> Self {
        let _old_record = self.put(record);
        self
    }

    /// Returns whether a record exists for an idempotency key.
    #[must_use]
    pub fn contains(&self, key: &IdempotencyKey) -> bool {
        self.records.contains_key(key)
    }

    /// Returns the number of stored records.
    #[must_use]
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Returns whether no records are stored.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Iterates over stored records in key order.
    pub fn iter(&self) -> impl Iterator<Item = &IdempotencyRecord> {
        self.records.values()
    }
}

impl FromIterator<IdempotencyRecord> for InMemoryIdempotencyStore {
    fn from_iter<T: IntoIterator<Item = IdempotencyRecord>>(iter: T) -> Self {
        let mut store = Self::new();
        for record in iter {
            let _old_record = store.put(record);
        }
        store
    }
}

impl IdempotencyStore for InMemoryIdempotencyStore {
    fn get(&self, key: &IdempotencyKey) -> IdempotencyStoreResult<Option<IdempotencyRecord>> {
        Ok(self.records.get(key).cloned())
    }

    fn put(
        &mut self,
        record: IdempotencyRecord,
    ) -> IdempotencyStoreResult<Option<IdempotencyRecord>> {
        Ok(self.records.insert(record.key().clone(), record))
    }

    fn remove(
        &mut self,
        key: &IdempotencyKey,
    ) -> IdempotencyStoreResult<Option<IdempotencyRecord>> {
        Ok(self.records.remove(key))
    }

    fn clear_expired(&mut self, now: SystemTime) -> IdempotencyStoreResult<usize> {
        let before = self.records.len();
        self.records.retain(|_, record| !record.is_expired_at(now));
        Ok(before - self.records.len())
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, UNIX_EPOCH};

    use super::{IdempotencyStore, InMemoryIdempotencyStore};
    use crate::{IdempotencyKey, IdempotencyRecord};

    #[test]
    fn in_memory_store_inserts_replaces_and_removes_records() {
        let key = IdempotencyKey::from("request-1");
        let old_record = IdempotencyRecord::in_progress_until(key.clone(), UNIX_EPOCH);
        let new_record =
            IdempotencyRecord::completed_until(key.clone(), UNIX_EPOCH + Duration::from_secs(60));
        let mut store = InMemoryIdempotencyStore::new();

        assert_eq!(
            store.put(old_record.clone()).expect("put should succeed"),
            None
        );
        assert_eq!(
            store.put(new_record.clone()).expect("put should succeed"),
            Some(old_record)
        );
        assert_eq!(
            store.get(&key).expect("get should succeed"),
            Some(new_record.clone())
        );
        assert!(store.contains(&key));
        assert_eq!(
            store.remove(&key).expect("remove should succeed"),
            Some(new_record)
        );
        assert!(store.is_empty());
    }

    #[test]
    fn in_memory_store_collects_records() {
        let first = IdempotencyRecord::in_progress_until("a", UNIX_EPOCH);
        let second = IdempotencyRecord::in_progress_until("b", UNIX_EPOCH);
        let store = InMemoryIdempotencyStore::from_records([second, first]);
        let keys = store
            .iter()
            .map(|record| record.key().value())
            .collect::<Vec<_>>();

        assert_eq!(keys, vec!["a", "b"]);
    }

    #[test]
    fn clear_expired_removes_only_expired_records() {
        let now = UNIX_EPOCH + Duration::from_secs(10);
        let expired = IdempotencyRecord::in_progress_until("expired", now);
        let active = IdempotencyRecord::in_progress_until("active", now + Duration::from_secs(1));
        let mut store = InMemoryIdempotencyStore::from_records([expired, active]);

        assert_eq!(store.clear_expired(now).expect("clear should succeed"), 1);
        assert_eq!(store.len(), 1);
        assert!(
            store
                .get(&IdempotencyKey::from("active"))
                .expect("get should succeed")
                .is_some()
        );
    }
}
