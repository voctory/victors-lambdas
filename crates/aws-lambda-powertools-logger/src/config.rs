//! Logger configuration.

use aws_lambda_powertools_core::{ServiceConfig, env};

use crate::LogLevel;

/// Configuration for structured logging.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoggerConfig {
    service: ServiceConfig,
    level: LogLevel,
    log_event: bool,
}

impl LoggerConfig {
    /// Creates logger configuration for a service name.
    #[must_use]
    pub fn new(service_name: impl Into<String>) -> Self {
        Self {
            service: ServiceConfig::new(service_name),
            level: LogLevel::Info,
            log_event: false,
        }
    }

    /// Creates logger configuration from environment variables.
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            service: ServiceConfig::from_env(),
            level: LogLevel::from_env(),
            log_event: env::bool_var(env::POWERTOOLS_LOGGER_LOG_EVENT),
        }
    }

    /// Returns a copy of the configuration with the given log level.
    #[must_use]
    pub fn with_level(mut self, level: LogLevel) -> Self {
        self.level = level;
        self
    }

    /// Returns a copy of the configuration with event logging enabled or disabled.
    #[must_use]
    pub fn with_event_logging(mut self, enabled: bool) -> Self {
        self.log_event = enabled;
        self
    }

    /// Returns the shared service configuration.
    #[must_use]
    pub fn service(&self) -> &ServiceConfig {
        &self.service
    }

    /// Returns the configured log level.
    #[must_use]
    pub fn level(&self) -> LogLevel {
        self.level
    }

    /// Returns whether incoming events should be logged.
    #[must_use]
    pub fn log_event(&self) -> bool {
        self.log_event
    }
}

impl Default for LoggerConfig {
    fn default() -> Self {
        Self::from_env()
    }
}
