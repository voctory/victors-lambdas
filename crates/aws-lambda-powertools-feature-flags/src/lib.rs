//! Feature flag evaluation utility.

#[cfg(feature = "appconfig")]
mod appconfig;
mod async_feature_flags;
mod config;
mod error;
mod feature_flags;
mod store;

#[cfg(feature = "appconfig")]
pub use appconfig::AppConfigFeatureFlagStore;
pub use async_feature_flags::{AsyncFeatureFlagStore, AsyncFeatureFlags, FeatureFlagFuture};
pub use config::{FeatureCondition, FeatureFlag, FeatureFlagConfig, FeatureRule, RuleAction};
pub use error::{FeatureFlagError, FeatureFlagErrorKind, FeatureFlagResult};
pub use feature_flags::{FeatureFlagContext, FeatureFlags};
pub use store::{FeatureFlagStore, InMemoryFeatureFlagStore};
