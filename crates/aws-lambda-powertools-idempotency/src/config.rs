//! Idempotency configuration.

use std::time::{Duration, SystemTime};

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
    key_prefix: Option<String>,
    record_ttl: Duration,
    in_progress_ttl: Duration,
    lambda_deadline: Option<SystemTime>,
}

impl IdempotencyConfig {
    /// Creates idempotency configuration.
    #[must_use]
    pub const fn new(disabled: bool) -> Self {
        Self {
            disabled,
            key_prefix: None,
            record_ttl: DEFAULT_RECORD_TTL,
            in_progress_ttl: DEFAULT_IN_PROGRESS_TTL,
            lambda_deadline: None,
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

    /// Returns the optional prefix applied to generated idempotency keys.
    #[must_use]
    pub fn key_prefix(&self) -> Option<&str> {
        self.key_prefix.as_deref()
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

    /// Returns the registered Lambda invocation deadline.
    ///
    /// When present, this deadline is used for in-progress record expiry so a
    /// retry can proceed after a timed-out invocation.
    #[must_use]
    pub const fn lambda_deadline(&self) -> Option<SystemTime> {
        self.lambda_deadline
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

    /// Returns a copy of this configuration with a Lambda invocation deadline.
    ///
    /// Pass `lambda_runtime::Context::deadline()` when using `lambda_runtime`.
    /// The deadline is used for in-progress record expiry.
    #[must_use]
    pub const fn with_lambda_deadline(mut self, deadline: SystemTime) -> Self {
        self.lambda_deadline = Some(deadline);
        self
    }

    /// Returns a copy of this configuration with Lambda remaining invocation time.
    ///
    /// The remaining time is converted to an absolute deadline when this method
    /// is called. For reusable workflows, register the current invocation's
    /// deadline or remaining time before each handler execution.
    #[must_use]
    pub fn with_lambda_remaining_time(self, remaining_time: Duration) -> Self {
        self.with_lambda_deadline(SystemTime::now() + remaining_time)
    }

    /// Registers a Lambda invocation deadline on this configuration.
    ///
    /// This should be refreshed for each Lambda invocation when the workflow is
    /// reused across invocations.
    pub fn register_lambda_deadline(&mut self, deadline: SystemTime) -> &mut Self {
        self.lambda_deadline = Some(deadline);
        self
    }

    /// Registers Lambda remaining invocation time on this configuration.
    ///
    /// The remaining time is converted to an absolute deadline when this method
    /// is called.
    pub fn register_lambda_remaining_time(&mut self, remaining_time: Duration) -> &mut Self {
        self.register_lambda_deadline(SystemTime::now() + remaining_time)
    }

    /// Clears a previously registered Lambda invocation deadline.
    pub fn clear_lambda_deadline(&mut self) -> &mut Self {
        self.lambda_deadline = None;
        self
    }

    /// Returns a copy of this configuration with an idempotency key prefix.
    #[must_use]
    pub fn with_key_prefix(mut self, key_prefix: impl Into<String>) -> Self {
        self.key_prefix = Some(key_prefix.into());
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
    use std::time::{Duration, UNIX_EPOCH};

    use super::{DEFAULT_IN_PROGRESS_TTL, DEFAULT_RECORD_TTL, IdempotencyConfig};

    #[test]
    fn new_uses_default_ttls() {
        let config = IdempotencyConfig::new(false);

        assert!(!config.disabled());
        assert_eq!(config.record_ttl(), DEFAULT_RECORD_TTL);
        assert_eq!(config.in_progress_ttl(), DEFAULT_IN_PROGRESS_TTL);
        assert_eq!(config.lambda_deadline(), None);
    }

    #[test]
    fn ttl_builders_replace_durations() {
        let config = IdempotencyConfig::new(true)
            .with_record_ttl(Duration::from_secs(10))
            .with_in_progress_ttl(Duration::from_secs(2))
            .with_key_prefix("orders");

        assert!(config.disabled());
        assert_eq!(config.key_prefix(), Some("orders"));
        assert_eq!(config.record_ttl(), Duration::from_secs(10));
        assert_eq!(config.in_progress_ttl(), Duration::from_secs(2));
    }

    #[test]
    fn lambda_deadline_can_be_registered_and_cleared() {
        let deadline = UNIX_EPOCH + Duration::from_secs(30);
        let mut config = IdempotencyConfig::new(false).with_lambda_deadline(deadline);

        assert_eq!(config.lambda_deadline(), Some(deadline));

        let next_deadline = UNIX_EPOCH + Duration::from_secs(60);
        config.register_lambda_deadline(next_deadline);
        assert_eq!(config.lambda_deadline(), Some(next_deadline));

        config.clear_lambda_deadline();
        assert_eq!(config.lambda_deadline(), None);
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
