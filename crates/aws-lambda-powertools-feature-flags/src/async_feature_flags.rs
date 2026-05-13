//! Asynchronous feature flag evaluator.

use std::{future::Future, pin::Pin};

use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;

use crate::{
    FeatureFlagConfig, FeatureFlagContext, FeatureFlagError, FeatureFlagResult, FeatureFlagStore,
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
#[derive(Clone, Debug, PartialEq)]
pub struct AsyncFeatureFlags<S> {
    store: S,
}

impl<S> AsyncFeatureFlags<S> {
    /// Creates an async feature flag evaluator with a store provider.
    #[must_use]
    pub const fn new(store: S) -> Self {
        Self { store }
    }

    /// Returns the configured store provider.
    #[must_use]
    pub const fn store(&self) -> &S {
        &self.store
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
        self.store.get_configuration().await
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
        let Ok(config) = self.store.get_configuration().await else {
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
        let Ok(config) = self.store.get_configuration().await else {
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
}

#[cfg(test)]
mod tests {
    use futures_executor::block_on;
    use serde_json::json;

    use super::AsyncFeatureFlags;
    use crate::{
        FeatureCondition, FeatureFlag, FeatureFlagConfig, FeatureFlagContext, FeatureRule,
        InMemoryFeatureFlagStore, RuleAction,
    };

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
}
