//! Logger type and log levels.

use aws_lambda_powertools_core::env;

use crate::LoggerConfig;

/// Log severity level.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LogLevel {
    /// Diagnostic trace logging.
    Trace,
    /// Diagnostic debug logging.
    Debug,
    /// Informational logging.
    Info,
    /// Warning logging.
    Warn,
    /// Error logging.
    Error,
}

impl LogLevel {
    /// Reads the log level from `POWERTOOLS_LOG_LEVEL`.
    #[must_use]
    pub fn from_env() -> Self {
        env::var(env::POWERTOOLS_LOG_LEVEL)
            .as_deref()
            .map_or(Self::Info, Self::from_name)
    }

    /// Parses a log level name.
    #[must_use]
    pub fn from_name(name: &str) -> Self {
        match name.trim().to_ascii_uppercase().as_str() {
            "TRACE" => Self::Trace,
            "DEBUG" => Self::Debug,
            "WARN" | "WARNING" => Self::Warn,
            "ERROR" => Self::Error,
            _ => Self::Info,
        }
    }
}

/// Structured logger facade.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Logger {
    config: LoggerConfig,
}

impl Logger {
    /// Creates a logger from environment configuration.
    #[must_use]
    pub fn new() -> Self {
        Self::with_config(LoggerConfig::from_env())
    }

    /// Creates a logger with explicit configuration.
    #[must_use]
    pub fn with_config(config: LoggerConfig) -> Self {
        Self { config }
    }

    /// Returns the logger configuration.
    #[must_use]
    pub fn config(&self) -> &LoggerConfig {
        &self.config
    }

    /// Returns the configured service name.
    #[must_use]
    pub fn service_name(&self) -> &str {
        self.config.service().service_name()
    }

    /// Returns the configured log level.
    #[must_use]
    pub fn level(&self) -> LogLevel {
        self.config.level()
    }

    /// Returns whether incoming events should be logged.
    #[must_use]
    pub fn logs_events(&self) -> bool {
        self.config.log_event()
    }
}

impl Default for Logger {
    fn default() -> Self {
        Self::new()
    }
}
