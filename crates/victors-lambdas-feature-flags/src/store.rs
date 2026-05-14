//! Feature flag store providers.

use crate::{FeatureFlagConfig, FeatureFlagResult};

/// Loads feature flag configuration from a backing store.
pub trait FeatureFlagStore {
    /// Gets the current feature flag configuration.
    ///
    /// # Errors
    ///
    /// Returns a store error when configuration cannot be loaded or decoded.
    fn get_configuration(&self) -> FeatureFlagResult<FeatureFlagConfig>;
}

/// In-memory feature flag store for tests and local examples.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct InMemoryFeatureFlagStore {
    config: FeatureFlagConfig,
}

impl InMemoryFeatureFlagStore {
    /// Creates an empty in-memory feature flag store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates an in-memory store from parsed feature flag configuration.
    #[must_use]
    pub fn from_config(config: FeatureFlagConfig) -> Self {
        Self { config }
    }

    /// Creates an in-memory store from a JSON configuration string.
    ///
    /// # Errors
    ///
    /// Returns a configuration error when the JSON document does not match the
    /// feature flag schema.
    pub fn from_json_str(input: &str) -> FeatureFlagResult<Self> {
        FeatureFlagConfig::from_json_str(input).map(Self::from_config)
    }

    /// Returns the stored configuration.
    #[must_use]
    pub const fn config(&self) -> &FeatureFlagConfig {
        &self.config
    }
}

impl FeatureFlagStore for InMemoryFeatureFlagStore {
    fn get_configuration(&self) -> FeatureFlagResult<FeatureFlagConfig> {
        Ok(self.config.clone())
    }
}
