//! Build and runtime metadata helpers.

/// Crate version for user-agent metadata.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Runtime identifier for user-agent metadata.
pub const RUNTIME: &str = "rust";

/// Builds a Powertools user-agent value.
#[must_use]
pub fn user_agent() -> String {
    format!("powertools-lambda-rust/{VERSION} ({RUNTIME})")
}
