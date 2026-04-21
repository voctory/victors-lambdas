//! Idempotency configuration.

use aws_lambda_powertools_core::env;

/// Environment variable that disables idempotency.
pub const POWERTOOLS_IDEMPOTENCY_DISABLED: &str = "POWERTOOLS_IDEMPOTENCY_DISABLED";

/// Configuration for idempotent handlers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IdempotencyConfig {
    disabled: bool,
}

impl IdempotencyConfig {
    /// Creates idempotency configuration.
    #[must_use]
    pub const fn new(disabled: bool) -> Self {
        Self { disabled }
    }

    /// Creates idempotency configuration from environment variables.
    #[must_use]
    pub fn from_env() -> Self {
        Self::new(env::bool_var(POWERTOOLS_IDEMPOTENCY_DISABLED))
    }

    /// Returns whether idempotency is disabled.
    #[must_use]
    pub const fn disabled(&self) -> bool {
        self.disabled
    }
}

impl Default for IdempotencyConfig {
    fn default() -> Self {
        Self::from_env()
    }
}
