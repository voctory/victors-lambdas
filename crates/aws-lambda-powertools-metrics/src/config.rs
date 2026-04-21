//! Metrics configuration.

use aws_lambda_powertools_core::{ServiceConfig, env};

/// Fallback `CloudWatch` namespace.
pub const DEFAULT_NAMESPACE: &str = "Powertools";

/// Configuration for metrics emission.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MetricsConfig {
    service: ServiceConfig,
    namespace: String,
    disabled: bool,
}

impl MetricsConfig {
    /// Creates metrics configuration for a service and namespace.
    #[must_use]
    pub fn new(service_name: impl Into<String>, namespace: impl Into<String>) -> Self {
        let namespace = namespace.into();
        let namespace = namespace.trim();
        let namespace = if namespace.is_empty() {
            DEFAULT_NAMESPACE
        } else {
            namespace
        };

        Self {
            service: ServiceConfig::new(service_name),
            namespace: namespace.to_owned(),
            disabled: false,
        }
    }

    /// Creates metrics configuration from environment variables.
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            service: ServiceConfig::from_env(),
            namespace: env::var_or(env::POWERTOOLS_METRICS_NAMESPACE, DEFAULT_NAMESPACE),
            disabled: env::bool_var(env::POWERTOOLS_METRICS_DISABLED),
        }
    }

    /// Returns a copy of the configuration with metrics disabled or enabled.
    #[must_use]
    pub fn with_disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Returns the shared service configuration.
    #[must_use]
    pub fn service(&self) -> &ServiceConfig {
        &self.service
    }

    /// Returns the metrics namespace.
    #[must_use]
    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    /// Returns whether metrics emission is disabled.
    #[must_use]
    pub fn disabled(&self) -> bool {
        self.disabled
    }
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self::from_env()
    }
}
