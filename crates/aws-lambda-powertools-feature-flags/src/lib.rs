//! Feature flag evaluation utility.

mod config;
mod error;
mod feature_flags;
mod store;

pub use config::{FeatureCondition, FeatureFlag, FeatureFlagConfig, FeatureRule, RuleAction};
pub use error::{FeatureFlagError, FeatureFlagErrorKind, FeatureFlagResult};
pub use feature_flags::{FeatureFlagContext, FeatureFlags};
pub use store::{FeatureFlagStore, InMemoryFeatureFlagStore};
