//! Environment variable names and parsing helpers.

/// Service name used by logger, metrics, and tracer utilities.
pub const POWERTOOLS_SERVICE_NAME: &str = "POWERTOOLS_SERVICE_NAME";

/// Log level for the logger utility.
pub const POWERTOOLS_LOG_LEVEL: &str = "POWERTOOLS_LOG_LEVEL";

/// Whether the logger should include the incoming event in logs.
pub const POWERTOOLS_LOGGER_LOG_EVENT: &str = "POWERTOOLS_LOGGER_LOG_EVENT";

/// Metrics namespace for `CloudWatch` EMF output.
pub const POWERTOOLS_METRICS_NAMESPACE: &str = "POWERTOOLS_METRICS_NAMESPACE";

/// Whether metrics output is disabled.
pub const POWERTOOLS_METRICS_DISABLED: &str = "POWERTOOLS_METRICS_DISABLED";

/// Whether tracing is enabled.
pub const POWERTOOLS_TRACE_ENABLED: &str = "POWERTOOLS_TRACE_ENABLED";

/// Returns a non-empty environment variable value.
#[must_use]
pub fn var(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|value| !value.is_empty())
}

/// Returns a non-empty environment variable value or a fallback.
#[must_use]
pub fn var_or(name: &str, fallback: &str) -> String {
    var(name).unwrap_or_else(|| fallback.to_owned())
}

/// Returns whether a string should be treated as enabled.
#[must_use]
pub fn is_truthy(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "y" | "on"
    )
}

/// Returns whether an environment variable should be treated as enabled.
#[must_use]
pub fn bool_var(name: &str) -> bool {
    var(name).is_some_and(|value| is_truthy(&value))
}
