//! Tracer configuration.

use aws_lambda_powertools_core::{ServiceConfig, env};

/// Configuration for tracing.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TracerConfig {
    service: ServiceConfig,
    enabled: bool,
}

impl TracerConfig {
    /// Creates tracer configuration for a service.
    #[must_use]
    pub fn new(service_name: impl Into<String>) -> Self {
        Self {
            service: ServiceConfig::new(service_name),
            enabled: true,
        }
    }

    /// Creates tracer configuration from environment variables.
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            service: ServiceConfig::from_env(),
            enabled: env::var(env::POWERTOOLS_TRACE_ENABLED)
                .is_none_or(|value| env::is_truthy(&value)),
        }
    }

    /// Returns a copy of the configuration with tracing enabled or disabled.
    #[must_use]
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Returns the shared service configuration.
    #[must_use]
    pub fn service(&self) -> &ServiceConfig {
        &self.service
    }

    /// Returns whether tracing is enabled.
    #[must_use]
    pub fn enabled(&self) -> bool {
        self.enabled
    }
}

impl Default for TracerConfig {
    fn default() -> Self {
        Self::from_env()
    }
}
