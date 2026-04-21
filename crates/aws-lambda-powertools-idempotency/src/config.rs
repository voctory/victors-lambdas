//! Idempotency configuration.

use std::time::Duration;

/// Environment variable that disables idempotency.
pub const POWERTOOLS_IDEMPOTENCY_DISABLED: &str = "POWERTOOLS_IDEMPOTENCY_DISABLED";

/// Default duration before completed records expire.
pub const DEFAULT_RECORD_TTL: Duration = Duration::from_secs(3_600);

/// Default duration before in-progress records expire.
pub const DEFAULT_IN_PROGRESS_TTL: Duration = Duration::from_secs(60);

/// Configuration for idempotent handlers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IdempotencyConfig {
    disabled: bool,
    record_ttl: Duration,
    in_progress_ttl: Duration,
}

impl IdempotencyConfig {
    /// Creates idempotency configuration.
    #[must_use]
    pub const fn new(disabled: bool) -> Self {
        Self {
            disabled,
            record_ttl: DEFAULT_RECORD_TTL,
            in_progress_ttl: DEFAULT_IN_PROGRESS_TTL,
        }
    }

    /// Creates idempotency configuration from environment variables.
    #[must_use]
    pub fn from_env() -> Self {
        Self::new(bool_var(POWERTOOLS_IDEMPOTENCY_DISABLED))
    }

    /// Returns whether idempotency is disabled.
    #[must_use]
    pub const fn disabled(&self) -> bool {
        self.disabled
    }

    /// Returns the completed record time-to-live duration.
    #[must_use]
    pub const fn record_ttl(&self) -> Duration {
        self.record_ttl
    }

    /// Returns the in-progress record time-to-live duration.
    #[must_use]
    pub const fn in_progress_ttl(&self) -> Duration {
        self.in_progress_ttl
    }

    /// Returns a copy of this configuration with a completed record time-to-live duration.
    #[must_use]
    pub const fn with_record_ttl(mut self, record_ttl: Duration) -> Self {
        self.record_ttl = record_ttl;
        self
    }

    /// Returns a copy of this configuration with an in-progress record time-to-live duration.
    #[must_use]
    pub const fn with_in_progress_ttl(mut self, in_progress_ttl: Duration) -> Self {
        self.in_progress_ttl = in_progress_ttl;
        self
    }
}

impl Default for IdempotencyConfig {
    fn default() -> Self {
        Self::from_env()
    }
}

fn bool_var(name: &str) -> bool {
    std::env::var(name)
        .ok()
        .is_some_and(|value| is_truthy(&value))
}

fn is_truthy(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "y" | "on"
    )
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{DEFAULT_IN_PROGRESS_TTL, DEFAULT_RECORD_TTL, IdempotencyConfig};

    #[test]
    fn new_uses_default_ttls() {
        let config = IdempotencyConfig::new(false);

        assert!(!config.disabled());
        assert_eq!(config.record_ttl(), DEFAULT_RECORD_TTL);
        assert_eq!(config.in_progress_ttl(), DEFAULT_IN_PROGRESS_TTL);
    }

    #[test]
    fn ttl_builders_replace_durations() {
        let config = IdempotencyConfig::new(true)
            .with_record_ttl(Duration::from_secs(10))
            .with_in_progress_ttl(Duration::from_secs(2));

        assert!(config.disabled());
        assert_eq!(config.record_ttl(), Duration::from_secs(10));
        assert_eq!(config.in_progress_ttl(), Duration::from_secs(2));
    }

    #[test]
    fn bool_var_accepts_truthy_tokens() {
        assert!(super::is_truthy("true"));
        assert!(super::is_truthy(" YES "));
        assert!(super::is_truthy("1"));
        assert!(!super::is_truthy("false"));
        assert!(!super::is_truthy(""));
    }
}
