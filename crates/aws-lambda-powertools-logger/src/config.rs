//! Logger configuration.

use aws_lambda_powertools_core::{ServiceConfig, env};

use crate::LogLevel;

const SAMPLE_RATE_SCALE: u32 = 1_000_000;

/// Configuration for structured logging.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoggerConfig {
    service: ServiceConfig,
    level: LogLevel,
    log_event: bool,
    sample_rate: u32,
}

impl LoggerConfig {
    /// Creates logger configuration for a service name.
    #[must_use]
    pub fn new(service_name: impl Into<String>) -> Self {
        Self {
            service: ServiceConfig::new(service_name),
            level: LogLevel::Info,
            log_event: false,
            sample_rate: 0,
        }
    }

    /// Creates logger configuration from environment variables.
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            service: ServiceConfig::from_env(),
            level: LogLevel::from_env(),
            log_event: env::bool_var(env::POWERTOOLS_LOGGER_LOG_EVENT),
            sample_rate: sample_rate_from_env(),
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

    /// Returns a copy of the configuration with debug log sampling.
    ///
    /// Values outside `0.0..=1.0` disable sampling.
    #[must_use]
    pub fn with_sample_rate(mut self, sample_rate: f64) -> Self {
        self.sample_rate = normalize_sample_rate(sample_rate);
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

    /// Returns the configured debug log sampling rate.
    #[must_use]
    pub fn sample_rate(&self) -> f64 {
        f64::from(self.sample_rate) / f64::from(SAMPLE_RATE_SCALE)
    }
}

impl Default for LoggerConfig {
    fn default() -> Self {
        Self::from_env()
    }
}

fn sample_rate_from_env() -> u32 {
    env::var(env::POWERTOOLS_LOGGER_SAMPLE_RATE)
        .and_then(|value| value.parse::<f64>().ok())
        .map_or(0, normalize_sample_rate)
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn normalize_sample_rate(sample_rate: f64) -> u32 {
    if !sample_rate.is_finite() || !(0.0..=1.0).contains(&sample_rate) {
        return 0;
    }

    (sample_rate * f64::from(SAMPLE_RATE_SCALE)).round() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_rates_are_normalized_to_supported_range() {
        assert!(
            (LoggerConfig::new("orders")
                .with_sample_rate(0.25)
                .sample_rate()
                - 0.25)
                .abs()
                < f64::EPSILON
        );
        assert!(
            LoggerConfig::new("orders")
                .with_sample_rate(1.5)
                .sample_rate()
                .abs()
                < f64::EPSILON
        );
        assert!(
            LoggerConfig::new("orders")
                .with_sample_rate(f64::NAN)
                .sample_rate()
                .abs()
                < f64::EPSILON
        );
    }
}
