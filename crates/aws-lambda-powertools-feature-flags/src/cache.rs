//! Cache policy values for feature flag configuration.

use std::time::{Duration, SystemTime};

use crate::FeatureFlagConfig;

/// Cache behavior for feature flag configuration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FeatureFlagCachePolicy {
    /// Do not cache feature flag configuration.
    Disabled,
    /// Cache feature flag configuration until explicitly cleared.
    Forever,
    /// Cache feature flag configuration for a fixed duration.
    Ttl(Duration),
}

impl FeatureFlagCachePolicy {
    /// Creates a disabled cache policy.
    #[must_use]
    pub const fn disabled() -> Self {
        Self::Disabled
    }

    /// Creates a cache policy that does not expire configuration automatically.
    #[must_use]
    pub const fn forever() -> Self {
        Self::Forever
    }

    /// Creates a cache policy with a time-to-live duration.
    #[must_use]
    pub const fn ttl(duration: Duration) -> Self {
        Self::Ttl(duration)
    }

    /// Returns whether this policy stores feature flag configuration.
    #[must_use]
    pub const fn enabled(self) -> bool {
        !matches!(self, Self::Disabled)
    }

    /// Returns the time-to-live duration for `Ttl` policies.
    #[must_use]
    pub const fn ttl_duration(self) -> Option<Duration> {
        match self {
            Self::Ttl(duration) => Some(duration),
            Self::Disabled | Self::Forever => None,
        }
    }

    /// Returns whether cached configuration is still fresh at `now`.
    #[must_use]
    pub fn is_fresh(self, cached_at: SystemTime, now: SystemTime) -> bool {
        match self {
            Self::Disabled => false,
            Self::Forever => true,
            Self::Ttl(duration) => now
                .duration_since(cached_at)
                .map_or(true, |age| age < duration),
        }
    }
}

impl Default for FeatureFlagCachePolicy {
    fn default() -> Self {
        Self::Disabled
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct CachedFeatureFlagConfig {
    pub(crate) config: FeatureFlagConfig,
    pub(crate) cached_at: SystemTime,
}

impl CachedFeatureFlagConfig {
    pub(crate) const fn new(config: FeatureFlagConfig, cached_at: SystemTime) -> Self {
        Self { config, cached_at }
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, UNIX_EPOCH};

    use super::FeatureFlagCachePolicy;

    #[test]
    fn disabled_policy_is_not_fresh() {
        let now = UNIX_EPOCH + Duration::from_secs(10);

        assert!(!FeatureFlagCachePolicy::disabled().enabled());
        assert!(!FeatureFlagCachePolicy::disabled().is_fresh(now, now));
    }

    #[test]
    fn forever_policy_is_always_fresh() {
        let cached_at = UNIX_EPOCH + Duration::from_secs(10);
        let later = UNIX_EPOCH + Duration::from_secs(3_600);

        assert!(FeatureFlagCachePolicy::forever().enabled());
        assert!(FeatureFlagCachePolicy::forever().is_fresh(cached_at, later));
    }

    #[test]
    fn ttl_policy_expires_at_duration_boundary() {
        let cached_at = UNIX_EPOCH + Duration::from_secs(10);
        let policy = FeatureFlagCachePolicy::ttl(Duration::from_secs(5));

        assert_eq!(policy.ttl_duration(), Some(Duration::from_secs(5)));
        assert!(policy.is_fresh(cached_at, cached_at + Duration::from_secs(4)));
        assert!(!policy.is_fresh(cached_at, cached_at + Duration::from_secs(5)));
    }
}
