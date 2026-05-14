//! Idempotency persistence traits and store adapters.

use std::{
    collections::BTreeMap,
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex, MutexGuard},
    time::SystemTime,
};

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

/// Local in-process cache wrapper for idempotency stores.
///
/// This wrapper keeps records in memory for the lifetime of the current Lambda
/// execution environment and delegates all persistence operations to the
/// wrapped store. It is useful in front of a durable store, such as `DynamoDB`, to
/// avoid repeated reads for warm retries while keeping the backing store as the
/// source of truth.
#[derive(Clone, Debug)]
pub struct CachedIdempotencyStore<S> {
    store: S,
    cache: Arc<Mutex<InMemoryIdempotencyStore>>,
}

impl<S> CachedIdempotencyStore<S> {
    /// Creates a cache wrapper around an idempotency store.
    #[must_use]
    pub fn new(store: S) -> Self {
        Self::with_cache(store, InMemoryIdempotencyStore::new())
    }

    /// Creates a cache wrapper with preloaded cache records.
    #[must_use]
    pub fn with_cache(store: S, cache: InMemoryIdempotencyStore) -> Self {
        Self {
            store,
            cache: Arc::new(Mutex::new(cache)),
        }
    }

    /// Returns the wrapped idempotency store.
    #[must_use]
    pub const fn store(&self) -> &S {
        &self.store
    }

    /// Returns a mutable reference to the wrapped idempotency store.
    pub const fn store_mut(&mut self) -> &mut S {
        &mut self.store
    }

    /// Consumes this wrapper and returns the wrapped idempotency store.
    #[must_use]
    pub fn into_store(self) -> S {
        self.store
    }

    /// Returns the number of records currently held in the local cache.
    ///
    /// # Errors
    ///
    /// Returns an error if the cache lock is poisoned.
    pub fn cache_len(&self) -> IdempotencyStoreResult<usize> {
        Ok(self.lock_cache()?.len())
    }

    /// Removes all records from the local cache.
    ///
    /// # Errors
    ///
    /// Returns an error if the cache lock is poisoned.
    pub fn clear_cache(&self) -> IdempotencyStoreResult<usize> {
        let mut cache = self.lock_cache()?;
        let removed = cache.len();
        *cache = InMemoryIdempotencyStore::new();
        Ok(removed)
    }

    fn lock_cache(&self) -> IdempotencyStoreResult<MutexGuard<'_, InMemoryIdempotencyStore>> {
        self.cache
            .lock()
            .map_err(|_| IdempotencyStoreError::new("idempotency local cache lock is poisoned"))
    }

    fn get_unexpired_cached_record(
        &self,
        key: &IdempotencyKey,
    ) -> IdempotencyStoreResult<Option<IdempotencyRecord>> {
        let mut cache = self.lock_cache()?;
        let record = cache.get(key)?;
        match record {
            Some(record) if record.is_expired_at(SystemTime::now()) => {
                cache.remove(key)?;
                Ok(None)
            }
            record => Ok(record),
        }
    }
}

impl<S> IdempotencyStore for CachedIdempotencyStore<S>
where
    S: IdempotencyStore,
{
    fn get(&self, key: &IdempotencyKey) -> IdempotencyStoreResult<Option<IdempotencyRecord>> {
        let cached = self.get_unexpired_cached_record(key)?;
        if cached.is_some() {
            return Ok(cached);
        }

        let record = self.store.get(key)?;
        if let Some(record) = record.clone() {
            self.lock_cache()?.put(record)?;
        }
        Ok(record)
    }

    fn put(
        &mut self,
        record: IdempotencyRecord,
    ) -> IdempotencyStoreResult<Option<IdempotencyRecord>> {
        let old_record = self.store.put(record.clone())?;
        self.lock_cache()?.put(record)?;
        Ok(old_record)
    }

    fn remove(
        &mut self,
        key: &IdempotencyKey,
    ) -> IdempotencyStoreResult<Option<IdempotencyRecord>> {
        let old_record = self.store.remove(key)?;
        self.lock_cache()?.remove(key)?;
        Ok(old_record)
    }

    fn clear_expired(&mut self, now: SystemTime) -> IdempotencyStoreResult<usize> {
        let removed = self.store.clear_expired(now)?;
        self.lock_cache()?.clear_expired(now)?;
        Ok(removed)
    }
}

impl<S> AsyncIdempotencyStore for CachedIdempotencyStore<S>
where
    S: AsyncIdempotencyStore,
{
    fn get<'a>(
        &'a self,
        key: &'a IdempotencyKey,
    ) -> IdempotencyStoreFuture<'a, Option<IdempotencyRecord>> {
        Box::pin(async move {
            let cached = self.get_unexpired_cached_record(key)?;
            if cached.is_some() {
                return Ok(cached);
            }

            let record = self.store.get(key).await?;
            if let Some(record) = record.clone() {
                self.lock_cache()?.put(record)?;
            }
            Ok(record)
        })
    }

    fn put(
        &self,
        record: IdempotencyRecord,
    ) -> IdempotencyStoreFuture<'_, Option<IdempotencyRecord>> {
        Box::pin(async move {
            let old_record = self.store.put(record.clone()).await?;
            self.lock_cache()?.put(record)?;
            Ok(old_record)
        })
    }

    fn remove<'a>(
        &'a self,
        key: &'a IdempotencyKey,
    ) -> IdempotencyStoreFuture<'a, Option<IdempotencyRecord>> {
        Box::pin(async move {
            let old_record = self.store.remove(key).await?;
            self.lock_cache()?.remove(key)?;
            Ok(old_record)
        })
    }

    fn clear_expired(&self, now: SystemTime) -> IdempotencyStoreFuture<'_, usize> {
        Box::pin(async move {
            let removed = self.store.clear_expired(now).await?;
            self.lock_cache()?.clear_expired(now)?;
            Ok(removed)
        })
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeMap,
        sync::{
            Mutex,
            atomic::{AtomicUsize, Ordering},
        },
        time::{Duration, SystemTime, UNIX_EPOCH},
    };

    use super::{
        AsyncIdempotencyStore, CachedIdempotencyStore, IdempotencyStore, IdempotencyStoreFuture,
        InMemoryIdempotencyStore,
    };
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

    #[test]
    fn cached_store_reads_through_and_caches_records() {
        let key = IdempotencyKey::from("request-1");
        let record = IdempotencyRecord::completed_until(key.clone(), future_expiry());
        let store = InMemoryIdempotencyStore::from_records([record.clone()]);
        let cached = CachedIdempotencyStore::new(store);

        assert_eq!(cached.cache_len().expect("cache length should load"), 0);
        assert_eq!(
            cached.get(&key).expect("get should succeed"),
            Some(record.clone())
        );
        assert_eq!(cached.cache_len().expect("cache length should load"), 1);
        assert_eq!(
            cached.get(&key).expect("get should hit cache"),
            Some(record)
        );
    }

    #[test]
    fn cached_store_writes_and_removes_from_cache_and_store() {
        let key = IdempotencyKey::from("request-1");
        let record = IdempotencyRecord::completed_until(key.clone(), future_expiry());
        let mut cached = CachedIdempotencyStore::new(InMemoryIdempotencyStore::new());

        assert_eq!(cached.put(record).expect("put should succeed"), None);
        assert!(cached.store().contains(&key));
        assert_eq!(cached.cache_len().expect("cache length should load"), 1);

        assert!(
            cached
                .remove(&key)
                .expect("remove should succeed")
                .is_some()
        );
        assert!(!cached.store().contains(&key));
        assert_eq!(cached.cache_len().expect("cache length should load"), 0);
    }

    #[test]
    fn cached_store_clears_expired_cache_records() {
        let now = UNIX_EPOCH + Duration::from_secs(10);
        let expired = IdempotencyRecord::in_progress_until("expired", now);
        let active = IdempotencyRecord::in_progress_until("active", now + Duration::from_secs(1));
        let mut cached = CachedIdempotencyStore::with_cache(
            InMemoryIdempotencyStore::from_records([expired.clone(), active.clone()]),
            InMemoryIdempotencyStore::from_records([expired, active]),
        );

        assert_eq!(cached.clear_expired(now).expect("clear should succeed"), 1);
        assert_eq!(cached.store().len(), 1);
        assert_eq!(cached.cache_len().expect("cache length should load"), 1);
    }

    #[test]
    fn cached_store_ignores_expired_cache_record_before_backing_store_read() {
        let key = IdempotencyKey::from("request-1");
        let expired = IdempotencyRecord::completed_until(key.clone(), UNIX_EPOCH);
        let fresh = IdempotencyRecord::completed_until(key.clone(), future_expiry());
        let cached = CachedIdempotencyStore::with_cache(
            InMemoryIdempotencyStore::from_records([fresh.clone()]),
            InMemoryIdempotencyStore::from_records([expired]),
        );

        assert_eq!(
            cached.get(&key).expect("get should read backing store"),
            Some(fresh)
        );
        assert_eq!(cached.cache_len().expect("cache length should load"), 1);
    }

    #[test]
    fn async_cached_store_reads_through_once() {
        let key = IdempotencyKey::from("request-1");
        let record = IdempotencyRecord::completed_until(key.clone(), future_expiry());
        let store = AsyncMemoryStore::from_records([record.clone()]);
        let cached = CachedIdempotencyStore::new(store);

        assert_eq!(
            futures_executor::block_on(cached.get(&key)).expect("get should succeed"),
            Some(record.clone())
        );
        assert_eq!(
            futures_executor::block_on(cached.get(&key)).expect("get should hit cache"),
            Some(record)
        );
        assert_eq!(cached.store().get_count(), 1);
    }

    fn future_expiry() -> SystemTime {
        SystemTime::now() + Duration::from_secs(60)
    }

    #[derive(Debug, Default)]
    struct AsyncMemoryStore {
        records: Mutex<BTreeMap<IdempotencyKey, IdempotencyRecord>>,
        get_count: AtomicUsize,
    }

    impl AsyncMemoryStore {
        fn from_records(records: impl IntoIterator<Item = IdempotencyRecord>) -> Self {
            Self {
                records: Mutex::new(
                    records
                        .into_iter()
                        .map(|record| (record.key().clone(), record))
                        .collect(),
                ),
                get_count: AtomicUsize::new(0),
            }
        }

        fn get_count(&self) -> usize {
            self.get_count.load(Ordering::Relaxed)
        }
    }

    impl AsyncIdempotencyStore for AsyncMemoryStore {
        fn get<'a>(
            &'a self,
            key: &'a IdempotencyKey,
        ) -> IdempotencyStoreFuture<'a, Option<IdempotencyRecord>> {
            Box::pin(async move {
                self.get_count.fetch_add(1, Ordering::Relaxed);
                Ok(self
                    .records
                    .lock()
                    .expect("records lock should not be poisoned")
                    .get(key)
                    .cloned())
            })
        }

        fn put(
            &self,
            record: IdempotencyRecord,
        ) -> IdempotencyStoreFuture<'_, Option<IdempotencyRecord>> {
            Box::pin(async move {
                Ok(self
                    .records
                    .lock()
                    .expect("records lock should not be poisoned")
                    .insert(record.key().clone(), record))
            })
        }

        fn remove<'a>(
            &'a self,
            key: &'a IdempotencyKey,
        ) -> IdempotencyStoreFuture<'a, Option<IdempotencyRecord>> {
            Box::pin(async move {
                Ok(self
                    .records
                    .lock()
                    .expect("records lock should not be poisoned")
                    .remove(key))
            })
        }

        fn clear_expired(&self, now: SystemTime) -> IdempotencyStoreFuture<'_, usize> {
            Box::pin(async move {
                let mut records = self
                    .records
                    .lock()
                    .expect("records lock should not be poisoned");
                let before = records.len();
                records.retain(|_, record| !record.is_expired_at(now));
                Ok(before - records.len())
            })
        }
    }
}
