//! Tracer configuration.

use victors_lambdas_core::{ServiceConfig, env};

/// Configuration for tracing.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TracerConfig {
    service: ServiceConfig,
    enabled: bool,
    capture_response: bool,
    capture_error: bool,
}

impl TracerConfig {
    /// Creates tracer configuration for a service.
    #[must_use]
    pub fn new(service_name: impl Into<String>) -> Self {
        Self {
            service: ServiceConfig::new(service_name),
            enabled: true,
            capture_response: true,
            capture_error: true,
        }
    }

    /// Creates tracer configuration from environment variables.
    #[must_use]
    pub fn from_env() -> Self {
        Self::from_env_source(env::var)
    }

    /// Creates tracer configuration from a custom environment source.
    ///
    /// This is useful for tests and for callers that keep configuration in an
    /// injected map instead of process globals.
    #[must_use]
    pub fn from_env_source(mut source: impl FnMut(&str) -> Option<String>) -> Self {
        Self {
            service: ServiceConfig::from_env_source(&mut source),
            enabled: trace_enabled_from_source(&mut source),
            capture_response: bool_from_source(
                &mut source,
                env::POWERTOOLS_TRACER_CAPTURE_RESPONSE,
                true,
            ),
            capture_error: bool_from_source(
                &mut source,
                env::POWERTOOLS_TRACER_CAPTURE_ERROR,
                true,
            ),
        }
    }

    /// Returns a copy of the configuration with tracing enabled or disabled.
    #[must_use]
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Returns a copy of the configuration with response capture enabled or disabled.
    #[must_use]
    pub fn with_capture_response(mut self, capture_response: bool) -> Self {
        self.capture_response = capture_response;
        self
    }

    /// Returns a copy of the configuration with error capture enabled or disabled.
    #[must_use]
    pub fn with_capture_error(mut self, capture_error: bool) -> Self {
        self.capture_error = capture_error;
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

    /// Returns whether handler responses should be captured.
    #[must_use]
    pub fn capture_response(&self) -> bool {
        self.capture_response
    }

    /// Returns whether handler errors should be captured.
    #[must_use]
    pub fn capture_error(&self) -> bool {
        self.capture_error
    }
}

impl Default for TracerConfig {
    fn default() -> Self {
        Self::from_env()
    }
}

fn trace_enabled_from_source(source: &mut impl FnMut(&str) -> Option<String>) -> bool {
    let enabled = bool_from_source(source, env::POWERTOOLS_TRACE_ENABLED, true);
    let disabled = bool_from_source(source, env::POWERTOOLS_TRACE_DISABLED, false);

    enabled && !disabled
}

fn bool_from_source(
    source: &mut impl FnMut(&str) -> Option<String>,
    name: &str,
    fallback: bool,
) -> bool {
    source(name)
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .and_then(|value| env::parse_bool(value).ok())
        .unwrap_or(fallback)
}

#[cfg(test)]
mod tests {
    use victors_lambdas_core::env;

    use super::TracerConfig;

    #[test]
    fn new_uses_default_tracer_flags() {
        let config = TracerConfig::new("  checkout  ");

        assert_eq!(config.service().service_name(), "checkout");
        assert!(config.enabled());
        assert!(config.capture_response());
        assert!(config.capture_error());
    }

    #[test]
    fn builders_update_tracer_flags() {
        let config = TracerConfig::new("orders")
            .with_enabled(false)
            .with_capture_response(false)
            .with_capture_error(false);

        assert!(!config.enabled());
        assert!(!config.capture_response());
        assert!(!config.capture_error());
    }

    #[test]
    fn from_env_source_reads_service_and_flags() {
        let config = TracerConfig::from_env_source(|name| match name {
            env::POWERTOOLS_SERVICE_NAME => Some("payments".to_owned()),
            env::POWERTOOLS_TRACE_ENABLED => Some("off".to_owned()),
            env::POWERTOOLS_TRACER_CAPTURE_RESPONSE => Some("false".to_owned()),
            env::POWERTOOLS_TRACER_CAPTURE_ERROR => Some("0".to_owned()),
            _ => None,
        });

        assert_eq!(config.service().service_name(), "payments");
        assert!(!config.enabled());
        assert!(!config.capture_response());
        assert!(!config.capture_error());
    }

    #[test]
    fn from_env_source_supports_trace_disabled_flag() {
        let config = TracerConfig::from_env_source(|name| match name {
            env::POWERTOOLS_SERVICE_NAME => Some("payments".to_owned()),
            env::POWERTOOLS_TRACE_DISABLED => Some("true".to_owned()),
            _ => None,
        });

        assert_eq!(config.service().service_name(), "payments");
        assert!(!config.enabled());
        assert!(config.capture_response());
        assert!(config.capture_error());
    }

    #[test]
    fn trace_disabled_flag_overrides_trace_enabled_flag() {
        let config = TracerConfig::from_env_source(|name| match name {
            env::POWERTOOLS_TRACE_ENABLED | env::POWERTOOLS_TRACE_DISABLED => {
                Some("true".to_owned())
            }
            _ => None,
        });

        assert!(!config.enabled());
    }

    #[test]
    fn from_env_source_uses_defaults_for_empty_or_invalid_flags() {
        let config = TracerConfig::from_env_source(|name| match name {
            env::POWERTOOLS_SERVICE_NAME => Some("orders".to_owned()),
            env::POWERTOOLS_TRACE_ENABLED => Some("   ".to_owned()),
            env::POWERTOOLS_TRACE_DISABLED | env::POWERTOOLS_TRACER_CAPTURE_RESPONSE => {
                Some("maybe".to_owned())
            }
            _ => None,
        });

        assert_eq!(config.service().service_name(), "orders");
        assert!(config.enabled());
        assert!(config.capture_response());
        assert!(config.capture_error());
    }
}
