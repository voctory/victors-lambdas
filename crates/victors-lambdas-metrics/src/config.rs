//! Metrics configuration.

use victors_lambdas_core::{ServiceConfig, env};

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
        Self::from_env_source(env::var)
    }

    /// Creates metrics configuration from a custom environment source.
    ///
    /// This is useful for tests and for callers that keep configuration in an
    /// injected map instead of process globals.
    #[must_use]
    pub fn from_env_source(mut source: impl FnMut(&str) -> Option<String>) -> Self {
        Self {
            service: ServiceConfig::from_env_source(&mut source),
            namespace: namespace_from_source(&mut source),
            disabled: disabled_from_source(&mut source),
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

fn namespace_from_source(source: &mut impl FnMut(&str) -> Option<String>) -> String {
    source(env::POWERTOOLS_METRICS_NAMESPACE)
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_NAMESPACE.to_owned())
}

fn disabled_from_source(source: &mut impl FnMut(&str) -> Option<String>) -> bool {
    if let Some(disabled) = source(env::POWERTOOLS_METRICS_DISABLED) {
        return env::parse_bool(&disabled).unwrap_or(false);
    }

    source(env::POWERTOOLS_DEV).is_some_and(|value| env::is_truthy(&value))
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self::from_env()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_trims_namespace_and_service_name() {
        let config = MetricsConfig::new("  checkout  ", "  Orders  ");

        assert_eq!(config.service().service_name(), "checkout");
        assert_eq!(config.namespace(), "Orders");
        assert!(!config.disabled());
    }

    #[test]
    fn new_uses_default_namespace_when_empty() {
        let config = MetricsConfig::new("checkout", "   ");

        assert_eq!(config.namespace(), DEFAULT_NAMESPACE);
    }

    #[test]
    fn with_disabled_updates_emission_flag() {
        let config = MetricsConfig::new("checkout", "Orders").with_disabled(true);

        assert!(config.disabled());
    }

    #[test]
    fn from_env_source_reads_service_namespace_and_disabled_flag() {
        let config = MetricsConfig::from_env_source(|name| match name {
            env::POWERTOOLS_SERVICE_NAME => Some("checkout".to_owned()),
            env::POWERTOOLS_METRICS_NAMESPACE => Some("Orders".to_owned()),
            env::POWERTOOLS_METRICS_DISABLED => Some("true".to_owned()),
            _ => None,
        });

        assert_eq!(config.service().service_name(), "checkout");
        assert_eq!(config.namespace(), "Orders");
        assert!(config.disabled());
    }

    #[test]
    fn from_env_source_uses_dev_mode_when_disabled_flag_is_absent() {
        let config = MetricsConfig::from_env_source(|name| {
            (name == env::POWERTOOLS_DEV).then(|| "true".to_owned())
        });

        assert!(config.disabled());
    }

    #[test]
    fn metrics_disabled_flag_overrides_dev_mode() {
        let config = MetricsConfig::from_env_source(|name| match name {
            env::POWERTOOLS_DEV => Some("true".to_owned()),
            env::POWERTOOLS_METRICS_DISABLED => Some("false".to_owned()),
            _ => None,
        });

        assert!(!config.disabled());
    }

    #[test]
    fn from_env_source_defaults_for_empty_or_invalid_values() {
        let config = MetricsConfig::from_env_source(|name| match name {
            env::POWERTOOLS_SERVICE_NAME | env::POWERTOOLS_METRICS_NAMESPACE => {
                Some("   ".to_owned())
            }
            env::POWERTOOLS_METRICS_DISABLED | env::POWERTOOLS_DEV => Some("maybe".to_owned()),
            _ => None,
        });

        assert_eq!(config.service().service_name(), "service_undefined");
        assert_eq!(config.namespace(), DEFAULT_NAMESPACE);
        assert!(!config.disabled());
    }
}
