//! Feature flag evaluation utility.

mod async_feature_flags;
mod config;
mod error;
mod feature_flags;
mod store;

pub use async_feature_flags::{AsyncFeatureFlagStore, AsyncFeatureFlags, FeatureFlagFuture};
pub use config::{FeatureCondition, FeatureFlag, FeatureFlagConfig, FeatureRule, RuleAction};
pub use error::{FeatureFlagError, FeatureFlagErrorKind, FeatureFlagResult};
pub use feature_flags::{FeatureFlagContext, FeatureFlags};
pub use store::{FeatureFlagStore, InMemoryFeatureFlagStore};
