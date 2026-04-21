//! Shared configuration values.

use crate::env;

/// Fallback service name used when no service name is configured.
pub const DEFAULT_SERVICE_NAME: &str = "service_undefined";

/// Configuration shared by utility crates.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServiceConfig {
    service_name: String,
}

impl ServiceConfig {
    /// Creates service configuration with a non-empty service name.
    #[must_use]
    pub fn new(service_name: impl Into<String>) -> Self {
        let service_name = service_name.into();
        let service_name = service_name.trim();
        let service_name = if service_name.is_empty() {
            DEFAULT_SERVICE_NAME
        } else {
            service_name
        };

        Self {
            service_name: service_name.to_owned(),
        }
    }

    /// Creates service configuration from environment variables.
    #[must_use]
    pub fn from_env() -> Self {
        Self::new(env::var_or(
            env::POWERTOOLS_SERVICE_NAME,
            DEFAULT_SERVICE_NAME,
        ))
    }

    /// Returns the configured service name.
    #[must_use]
    pub fn service_name(&self) -> &str {
        &self.service_name
    }
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self::from_env()
    }
}
