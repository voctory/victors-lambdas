//! Cache policy values for parameter retrieval.

use std::time::{Duration, SystemTime};

/// Cache behavior for resolved parameter values.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CachePolicy {
    /// Do not cache parameter values.
    Disabled,
    /// Cache parameter values until explicitly cleared.
    Forever,
    /// Cache parameter values for a fixed duration.
    Ttl(Duration),
}

impl CachePolicy {
    /// Creates a disabled cache policy.
    #[must_use]
    pub const fn disabled() -> Self {
        Self::Disabled
    }

    /// Creates a cache policy that does not expire values automatically.
    #[must_use]
    pub const fn forever() -> Self {
        Self::Forever
    }

    /// Creates a cache policy with a time-to-live duration.
    #[must_use]
    pub const fn ttl(duration: Duration) -> Self {
        Self::Ttl(duration)
    }

    /// Returns whether this policy stores parameter values.
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

    /// Returns whether a cached value is still fresh at `now`.
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

impl Default for CachePolicy {
    fn default() -> Self {
        Self::Disabled
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, UNIX_EPOCH};

    use super::CachePolicy;

    #[test]
    fn disabled_policy_is_not_fresh() {
        let now = UNIX_EPOCH + Duration::from_secs(10);

        assert!(!CachePolicy::disabled().enabled());
        assert!(!CachePolicy::disabled().is_fresh(now, now));
    }

    #[test]
    fn forever_policy_is_always_fresh() {
        let cached_at = UNIX_EPOCH + Duration::from_secs(10);
        let later = UNIX_EPOCH + Duration::from_secs(3_600);

        assert!(CachePolicy::forever().enabled());
        assert!(CachePolicy::forever().is_fresh(cached_at, later));
    }

    #[test]
    fn ttl_policy_expires_at_duration_boundary() {
        let cached_at = UNIX_EPOCH + Duration::from_secs(10);
        let policy = CachePolicy::ttl(Duration::from_secs(5));

        assert_eq!(policy.ttl_duration(), Some(Duration::from_secs(5)));
        assert!(policy.is_fresh(cached_at, cached_at + Duration::from_secs(4)));
        assert!(!policy.is_fresh(cached_at, cached_at + Duration::from_secs(5)));
    }
}
