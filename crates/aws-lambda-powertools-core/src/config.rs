//! Shared configuration values.

use crate::env;

/// Fallback service name used when no service name is configured.
pub const DEFAULT_SERVICE_NAME: &str = "service_undefined";

/// Configuration shared by utility crates.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServiceConfig {
    service_name: String,
}

/// Builder for [`ServiceConfig`].
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ServiceConfigBuilder {
    service_name: Option<String>,
}

impl ServiceConfig {
    /// Creates service configuration with a non-empty service name.
    #[must_use]
    pub fn new(service_name: impl Into<String>) -> Self {
        Self::builder()
            .service_name(service_name)
            .build_without_env()
    }

    /// Creates a builder that uses environment variables for unspecified values.
    #[must_use]
    pub fn builder() -> ServiceConfigBuilder {
        ServiceConfigBuilder::default()
    }

    /// Creates service configuration from environment variables.
    #[must_use]
    pub fn from_env() -> Self {
        Self::builder().build()
    }

    /// Creates service configuration from a custom environment source.
    ///
    /// This is useful for tests and for callers that keep configuration in an injected environment
    /// map instead of process globals.
    #[must_use]
    pub fn from_env_source(source: impl FnMut(&str) -> Option<String>) -> Self {
        Self::builder().build_with_env(source)
    }

    /// Returns a copy of the configuration with an explicit service name.
    #[must_use]
    pub fn with_service_name(mut self, service_name: impl Into<String>) -> Self {
        self.service_name =
            normalize_service_name(service_name).unwrap_or_else(default_service_name);
        self
    }

    /// Returns the configured service name.
    #[must_use]
    pub fn service_name(&self) -> &str {
        &self.service_name
    }
}

impl ServiceConfigBuilder {
    /// Overrides the service name.
    #[must_use]
    pub fn service_name(mut self, service_name: impl Into<String>) -> Self {
        self.service_name =
            Some(normalize_service_name(service_name).unwrap_or_else(default_service_name));
        self
    }

    /// Builds service configuration using process environment variables for unspecified values.
    #[must_use]
    pub fn build(self) -> ServiceConfig {
        self.build_with_env(env::var)
    }

    /// Builds service configuration using a custom environment source for unspecified values.
    #[must_use]
    pub fn build_with_env(
        mut self,
        mut source: impl FnMut(&str) -> Option<String>,
    ) -> ServiceConfig {
        let service_name = self
            .service_name
            .take()
            .or_else(|| source(env::POWERTOOLS_SERVICE_NAME).and_then(normalize_service_name))
            .unwrap_or_else(default_service_name);

        ServiceConfig { service_name }
    }

    /// Builds service configuration without reading environment variables.
    #[must_use]
    pub fn build_without_env(mut self) -> ServiceConfig {
        let service_name = self
            .service_name
            .take()
            .unwrap_or_else(default_service_name);

        ServiceConfig { service_name }
    }
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self::from_env()
    }
}

fn normalize_service_name(service_name: impl Into<String>) -> Option<String> {
    let service_name = service_name.into();
    let service_name = service_name.trim();
    (!service_name.is_empty()).then(|| service_name.to_owned())
}

fn default_service_name() -> String {
    DEFAULT_SERVICE_NAME.to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_trims_service_name_without_reading_env() {
        let config = ServiceConfig::new("  checkout  ");

        assert_eq!(config.service_name(), "checkout");
    }

    #[test]
    fn new_uses_default_for_empty_service_name() {
        let config = ServiceConfig::new("   ");

        assert_eq!(config.service_name(), DEFAULT_SERVICE_NAME);
    }

    #[test]
    fn builder_uses_env_when_override_is_missing() {
        let config = ServiceConfig::builder().build_with_env(|name| {
            (name == env::POWERTOOLS_SERVICE_NAME).then(|| "orders".to_owned())
        });

        assert_eq!(config.service_name(), "orders");
    }

    #[test]
    fn builder_prefers_override_to_env() {
        let config = ServiceConfig::builder()
            .service_name("payments")
            .build_with_env(|_| Some("orders".to_owned()));

        assert_eq!(config.service_name(), "payments");
    }

    #[test]
    fn builder_uses_default_when_override_and_env_are_empty() {
        let config = ServiceConfig::builder()
            .service_name("")
            .build_with_env(|_| Some("   ".to_owned()));

        assert_eq!(config.service_name(), DEFAULT_SERVICE_NAME);
    }

    #[test]
    fn builder_empty_override_does_not_fall_back_to_env() {
        let config = ServiceConfig::builder()
            .service_name("")
            .build_with_env(|_| Some("orders".to_owned()));

        assert_eq!(config.service_name(), DEFAULT_SERVICE_NAME);
    }

    #[test]
    fn with_service_name_updates_existing_config() {
        let config = ServiceConfig::new("orders").with_service_name("payments");

        assert_eq!(config.service_name(), "payments");
    }
}
