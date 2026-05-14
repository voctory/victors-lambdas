//! Asynchronous feature flag evaluator.

use std::{
    future::Future,
    pin::Pin,
    sync::{Mutex, PoisonError},
    time::SystemTime,
};

use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;

use crate::{
    FeatureFlagCachePolicy, FeatureFlagConfig, FeatureFlagContext, FeatureFlagError,
    FeatureFlagResult, FeatureFlagStore, cache::CachedFeatureFlagConfig,
    feature_flags::evaluate_feature,
};

/// Boxed future returned by asynchronous feature flag stores.
pub type FeatureFlagFuture<'a> =
    Pin<Box<dyn Future<Output = FeatureFlagResult<FeatureFlagConfig>> + Send + 'a>>;

/// Loads feature flag configuration asynchronously from a backing store.
pub trait AsyncFeatureFlagStore: Sync {
    /// Gets the current feature flag configuration.
    fn get_configuration(&self) -> FeatureFlagFuture<'_>;
}

impl<T> AsyncFeatureFlagStore for T
where
    T: FeatureFlagStore + Sync,
{
    fn get_configuration(&self) -> FeatureFlagFuture<'_> {
        Box::pin(async move { FeatureFlagStore::get_configuration(self) })
    }
}

/// Asynchronously evaluates feature flags against a configured store.
#[derive(Debug)]
pub struct AsyncFeatureFlags<S> {
    store: S,
    cache_policy: FeatureFlagCachePolicy,
    cache: Mutex<Option<CachedFeatureFlagConfig>>,
}

impl<S> AsyncFeatureFlags<S> {
    /// Creates an async feature flag evaluator with a store provider.
    #[must_use]
    pub fn new(store: S) -> Self {
        Self::with_cache_policy(store, FeatureFlagCachePolicy::default())
    }

    /// Creates an async feature flag evaluator with a store provider and cache policy.
    #[must_use]
    pub const fn with_cache_policy(store: S, cache_policy: FeatureFlagCachePolicy) -> Self {
        Self {
            store,
            cache_policy,
            cache: Mutex::new(None),
        }
    }

    /// Returns the configured store provider.
    #[must_use]
    pub const fn store(&self) -> &S {
        &self.store
    }

    /// Returns the feature flag configuration cache policy.
    #[must_use]
    pub const fn cache_policy(&self) -> FeatureFlagCachePolicy {
        self.cache_policy
    }

    /// Clears cached feature flag configuration.
    pub fn clear_cache(&self) {
        self.cache
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
    }

    /// Returns the number of cached feature flag configurations.
    #[must_use]
    pub fn cache_len(&self) -> usize {
        usize::from(
            self.cache
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .is_some(),
        )
    }
}

impl<S> AsyncFeatureFlags<S>
where
    S: AsyncFeatureFlagStore,
{
    /// Gets the feature flag configuration from the configured store.
    ///
    /// # Errors
    ///
    /// Returns a store or configuration error when the store cannot provide a
    /// feature flag configuration.
    pub async fn get_configuration(&self) -> FeatureFlagResult<FeatureFlagConfig> {
        self.get_configuration_at(SystemTime::now()).await
    }

    /// Gets feature flag configuration from the store, bypassing any cached value.
    ///
    /// When the store returns configuration, the cache is updated with that
    /// value if caching is enabled.
    ///
    /// # Errors
    ///
    /// Returns a store or configuration error when the store cannot provide a
    /// feature flag configuration.
    pub async fn get_configuration_force(&self) -> FeatureFlagResult<FeatureFlagConfig> {
        self.fetch_configuration_at(SystemTime::now()).await
    }

    /// Evaluates a feature and returns its JSON value.
    ///
    /// If the store cannot load configuration or the feature does not exist,
    /// this returns the provided default value.
    pub async fn evaluate_value(
        &self,
        name: &str,
        context: &FeatureFlagContext,
        default: Value,
    ) -> Value {
        let Ok(config) = self.get_configuration().await else {
            return default;
        };

        config
            .get(name)
            .map_or(default, |feature| evaluate_feature(feature, context))
    }

    /// Evaluates a boolean feature.
    ///
    /// If the store cannot load configuration or the feature does not exist,
    /// this returns the provided default value.
    ///
    /// # Errors
    ///
    /// Returns a transform error when the matched feature value is not a JSON
    /// boolean.
    pub async fn evaluate_bool(
        &self,
        name: &str,
        context: &FeatureFlagContext,
        default: bool,
    ) -> FeatureFlagResult<bool> {
        let value = self
            .evaluate_value(name, context, Value::Bool(default))
            .await;
        value.as_bool().ok_or_else(|| {
            FeatureFlagError::transform(name, format!("feature value is not a boolean: {value}"))
        })
    }

    /// Evaluates a feature and deserializes the JSON value into the requested type.
    ///
    /// If the store cannot load configuration or the feature does not exist,
    /// this deserializes the provided default value.
    ///
    /// # Errors
    ///
    /// Returns a transform error when the default cannot be serialized or the
    /// evaluated feature value cannot be deserialized into the requested type.
    pub async fn evaluate_json<T>(
        &self,
        name: &str,
        context: &FeatureFlagContext,
        default: T,
    ) -> FeatureFlagResult<T>
    where
        T: DeserializeOwned + Serialize,
    {
        let default = serde_json::to_value(default).map_err(|error| {
            FeatureFlagError::transform(name, format!("default value is not JSON: {error}"))
        })?;
        let value = self.evaluate_value(name, context, default).await;

        serde_json::from_value(value).map_err(|error| {
            FeatureFlagError::transform(
                name,
                format!("feature value has unexpected shape: {error}"),
            )
        })
    }

    /// Returns boolean features enabled for the provided context.
    ///
    /// Store failures return an empty list.
    pub async fn get_enabled_features(&self, context: &FeatureFlagContext) -> Vec<String> {
        let Ok(config) = self.get_configuration().await else {
            return Vec::new();
        };

        config
            .iter()
            .filter_map(|(name, feature)| {
                evaluate_feature(feature, context)
                    .as_bool()
                    .filter(|enabled| *enabled)
                    .map(|_| name.to_owned())
            })
            .collect()
    }

    async fn get_configuration_at(&self, now: SystemTime) -> FeatureFlagResult<FeatureFlagConfig> {
        if let Some(config) = self.cached_configuration(now) {
            return Ok(config);
        }

        self.fetch_configuration_at(now).await
    }

    async fn fetch_configuration_at(
        &self,
        now: SystemTime,
    ) -> FeatureFlagResult<FeatureFlagConfig> {
        let config = self.store.get_configuration().await?;
        self.store_cached_configuration(config.clone(), now);
        Ok(config)
    }

    fn cached_configuration(&self, now: SystemTime) -> Option<FeatureFlagConfig> {
        if !self.cache_policy.enabled() {
            return None;
        }

        self.cache
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .as_ref()
            .filter(|cached| self.cache_policy.is_fresh(cached.cached_at, now))
            .map(|cached| cached.config.clone())
    }

    fn store_cached_configuration(&self, config: FeatureFlagConfig, now: SystemTime) {
        if !self.cache_policy.enabled() {
            return;
        }

        self.cache
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .replace(CachedFeatureFlagConfig::new(config, now));
    }
}

impl<S> Clone for AsyncFeatureFlags<S>
where
    S: Clone,
{
    fn clone(&self) -> Self {
        Self {
            store: self.store.clone(),
            cache_policy: self.cache_policy,
            cache: Mutex::new(
                self.cache
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .clone(),
            ),
        }
    }
}

impl<S> PartialEq for AsyncFeatureFlags<S>
where
    S: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.store == other.store && self.cache_policy == other.cache_policy
    }
}

impl<S> Eq for AsyncFeatureFlags<S> where S: Eq {}

#[cfg(test)]
mod tests {
    use std::{
        sync::{Arc, Mutex},
        time::Duration,
    };

    use futures_executor::block_on;
    use serde_json::json;

    use super::AsyncFeatureFlags;
    use crate::{
        FeatureCondition, FeatureFlag, FeatureFlagCachePolicy, FeatureFlagConfig,
        FeatureFlagContext, FeatureFlagResult, FeatureFlagStore, FeatureRule,
        InMemoryFeatureFlagStore, RuleAction,
    };

    #[derive(Clone, Debug)]
    struct CountingStore {
        config: FeatureFlagConfig,
        calls: Arc<Mutex<usize>>,
    }

    impl CountingStore {
        fn new(config: FeatureFlagConfig) -> Self {
            Self {
                config,
                calls: Arc::new(Mutex::new(0)),
            }
        }

        fn calls(&self) -> usize {
            *self.calls.lock().unwrap()
        }
    }

    impl FeatureFlagStore for CountingStore {
        fn get_configuration(&self) -> FeatureFlagResult<FeatureFlagConfig> {
            *self.calls.lock().unwrap() += 1;
            Ok(self.config.clone())
        }
    }

    #[test]
    fn async_evaluator_uses_sync_stores() {
        let feature = FeatureFlag::boolean(false).with_rule(
            "premium tier",
            FeatureRule::new(
                true,
                [FeatureCondition::new(RuleAction::Equals, "tier", "premium")],
            ),
        );
        let flags = AsyncFeatureFlags::new(InMemoryFeatureFlagStore::from_config(
            FeatureFlagConfig::new().with_feature("premium_features", feature),
        ));
        let context = FeatureFlagContext::from_iter([("tier".to_owned(), json!("premium"))]);

        assert!(block_on(flags.evaluate_bool("premium_features", &context, false)).unwrap());
        assert_eq!(
            block_on(flags.get_enabled_features(&context)),
            vec!["premium_features"]
        );
    }

    #[test]
    fn async_evaluator_reuses_cached_configuration() {
        let store = CountingStore::new(
            FeatureFlagConfig::new().with_feature("always_on", FeatureFlag::boolean(true)),
        );
        let flags = AsyncFeatureFlags::with_cache_policy(
            store.clone(),
            FeatureFlagCachePolicy::ttl(Duration::from_secs(60)),
        );

        assert!(
            block_on(flags.evaluate_bool("always_on", &FeatureFlagContext::new(), false)).unwrap()
        );
        assert!(
            block_on(flags.evaluate_bool("always_on", &FeatureFlagContext::new(), false)).unwrap()
        );
        assert_eq!(store.calls(), 1);
        assert_eq!(flags.cache_len(), 1);

        block_on(flags.get_configuration_force()).unwrap();
        assert_eq!(store.calls(), 2);
    }
}
