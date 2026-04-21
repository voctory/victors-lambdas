//! Environment variable names and parsing helpers.

use std::{error::Error, fmt, time::Duration};

/// Service name used by logger, metrics, and tracer utilities.
pub const POWERTOOLS_SERVICE_NAME: &str = "POWERTOOLS_SERVICE_NAME";

/// Log level for the logger utility.
pub const POWERTOOLS_LOG_LEVEL: &str = "POWERTOOLS_LOG_LEVEL";

/// Whether the logger should include the incoming event in logs.
pub const POWERTOOLS_LOGGER_LOG_EVENT: &str = "POWERTOOLS_LOGGER_LOG_EVENT";

/// Sampling rate for debug log sampling.
pub const POWERTOOLS_LOGGER_SAMPLE_RATE: &str = "POWERTOOLS_LOGGER_SAMPLE_RATE";

/// Metrics namespace for `CloudWatch` EMF output.
pub const POWERTOOLS_METRICS_NAMESPACE: &str = "POWERTOOLS_METRICS_NAMESPACE";

/// Whether metrics output is disabled.
pub const POWERTOOLS_METRICS_DISABLED: &str = "POWERTOOLS_METRICS_DISABLED";

/// Function name override for metrics metadata.
pub const POWERTOOLS_METRICS_FUNCTION_NAME: &str = "POWERTOOLS_METRICS_FUNCTION_NAME";

/// Whether tracing is enabled.
pub const POWERTOOLS_TRACE_ENABLED: &str = "POWERTOOLS_TRACE_ENABLED";

/// Whether tracer utilities should capture handler responses.
pub const POWERTOOLS_TRACER_CAPTURE_RESPONSE: &str = "POWERTOOLS_TRACER_CAPTURE_RESPONSE";

/// Whether tracer utilities should capture handler errors.
pub const POWERTOOLS_TRACER_CAPTURE_ERROR: &str = "POWERTOOLS_TRACER_CAPTURE_ERROR";

/// Maximum age for cached parameter provider values.
pub const POWERTOOLS_PARAMETERS_MAX_AGE: &str = "POWERTOOLS_PARAMETERS_MAX_AGE";

/// Whether the `SSM` parameter provider should decrypt secure strings.
pub const POWERTOOLS_PARAMETERS_SSM_DECRYPT: &str = "POWERTOOLS_PARAMETERS_SSM_DECRYPT";

/// Whether idempotency behavior is disabled.
pub const POWERTOOLS_IDEMPOTENCY_DISABLED: &str = "POWERTOOLS_IDEMPOTENCY_DISABLED";

/// Whether Powertools should use development-friendly behavior.
pub const POWERTOOLS_DEV: &str = "POWERTOOLS_DEV";

/// Whether Powertools debug behavior is enabled.
pub const POWERTOOLS_DEBUG: &str = "POWERTOOLS_DEBUG";

/// Error returned when a typed environment value cannot be parsed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnvParseError {
    name: Option<String>,
    value: String,
    expected: &'static str,
}

impl EnvParseError {
    /// Creates an error for a raw value.
    #[must_use]
    pub fn new(value: impl Into<String>, expected: &'static str) -> Self {
        Self {
            name: None,
            value: value.into(),
            expected,
        }
    }

    /// Creates an error for a named environment variable.
    #[must_use]
    pub fn for_var(
        name: impl Into<String>,
        value: impl Into<String>,
        expected: &'static str,
    ) -> Self {
        Self {
            name: Some(name.into()),
            value: value.into(),
            expected,
        }
    }

    /// Returns the environment variable name when one is available.
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Returns the invalid value.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Returns a short description of the expected value format.
    #[must_use]
    pub fn expected(&self) -> &'static str {
        self.expected
    }

    fn with_name(mut self, name: &str) -> Self {
        self.name = Some(name.to_owned());
        self
    }
}

impl fmt::Display for EnvParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(name) = self.name() {
            write!(
                formatter,
                "invalid value {:?} for environment variable {name}; expected {}",
                self.value, self.expected
            )
        } else {
            write!(
                formatter,
                "invalid environment value {:?}; expected {}",
                self.value, self.expected
            )
        }
    }
}

impl Error for EnvParseError {}

/// Returns a trimmed, non-empty environment variable value.
#[must_use]
pub fn var(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .and_then(|value| normalize_string(&value))
}

/// Returns a trimmed, non-empty environment variable value or a fallback.
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

/// Returns whether a string should be treated as disabled.
#[must_use]
pub fn is_falsy(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "0" | "false" | "no" | "n" | "off"
    )
}

/// Parses a boolean value.
///
/// Accepted enabled values are `1`, `true`, `yes`, `y`, and `on`. Accepted disabled values are
/// `0`, `false`, `no`, `n`, and `off`. Matching is case-insensitive and ignores surrounding
/// whitespace.
///
/// # Errors
///
/// Returns [`EnvParseError`] when the value is empty or does not match a supported boolean token.
pub fn parse_bool(value: &str) -> Result<bool, EnvParseError> {
    if is_truthy(value) {
        Ok(true)
    } else if is_falsy(value) {
        Ok(false)
    } else {
        Err(EnvParseError::new(value, "a boolean"))
    }
}

/// Parses an optional boolean environment variable.
///
/// # Errors
///
/// Returns [`EnvParseError`] when the variable is set to a non-empty value that does not match a
/// supported boolean token.
pub fn try_bool_var(name: &str) -> Result<Option<bool>, EnvParseError> {
    var(name)
        .map(|value| parse_bool(&value).map_err(|error| error.with_name(name)))
        .transpose()
}

/// Returns whether an environment variable should be treated as enabled.
///
/// Invalid or unset values are treated as `false`. Use [`try_bool_var`] when callers need to reject
/// invalid configuration.
#[must_use]
pub fn bool_var(name: &str) -> bool {
    try_bool_var(name).ok().flatten().unwrap_or(false)
}

/// Returns a boolean environment variable value or a fallback.
///
/// Invalid values use the fallback. Use [`try_bool_var`] when callers need to reject invalid
/// configuration.
#[must_use]
pub fn bool_var_or(name: &str, fallback: bool) -> bool {
    try_bool_var(name).ok().flatten().unwrap_or(fallback)
}

/// Parses a signed 64-bit integer value.
///
/// # Errors
///
/// Returns [`EnvParseError`] when the value is empty or cannot be parsed as an `i64`.
pub fn parse_i64(value: &str) -> Result<i64, EnvParseError> {
    let value = value.trim();

    if value.is_empty() {
        return Err(EnvParseError::new(value, "an i64 integer"));
    }

    value
        .parse()
        .map_err(|_| EnvParseError::new(value, "an i64 integer"))
}

/// Parses an unsigned 64-bit integer value.
///
/// # Errors
///
/// Returns [`EnvParseError`] when the value is empty or cannot be parsed as a `u64`.
pub fn parse_u64(value: &str) -> Result<u64, EnvParseError> {
    let value = value.trim();

    if value.is_empty() {
        return Err(EnvParseError::new(value, "a u64 integer"));
    }

    value
        .parse()
        .map_err(|_| EnvParseError::new(value, "a u64 integer"))
}

/// Parses an optional signed integer environment variable.
///
/// # Errors
///
/// Returns [`EnvParseError`] when the variable is set to a non-empty value that cannot be parsed as
/// an `i64`.
pub fn i64_var(name: &str) -> Result<Option<i64>, EnvParseError> {
    var(name)
        .map(|value| parse_i64(&value).map_err(|error| error.with_name(name)))
        .transpose()
}

/// Parses a signed integer environment variable or returns a fallback.
///
/// # Errors
///
/// Returns [`EnvParseError`] when the variable is set to a non-empty value that cannot be parsed as
/// an `i64`.
pub fn i64_var_or(name: &str, fallback: i64) -> Result<i64, EnvParseError> {
    i64_var(name).map(|value| value.unwrap_or(fallback))
}

/// Parses an optional unsigned integer environment variable.
///
/// # Errors
///
/// Returns [`EnvParseError`] when the variable is set to a non-empty value that cannot be parsed as
/// a `u64`.
pub fn u64_var(name: &str) -> Result<Option<u64>, EnvParseError> {
    var(name)
        .map(|value| parse_u64(&value).map_err(|error| error.with_name(name)))
        .transpose()
}

/// Parses an unsigned integer environment variable or returns a fallback.
///
/// # Errors
///
/// Returns [`EnvParseError`] when the variable is set to a non-empty value that cannot be parsed as
/// a `u64`.
pub fn u64_var_or(name: &str, fallback: u64) -> Result<u64, EnvParseError> {
    u64_var(name).map(|value| value.unwrap_or(fallback))
}

/// Parses a duration value.
///
/// Bare integer values are interpreted as seconds. Supported suffixes are `ms`, `s`, `m`, `h`, and
/// `d`, with long forms such as `seconds` and `minutes` also accepted.
///
/// # Errors
///
/// Returns [`EnvParseError`] when the value is empty, negative, uses an unknown unit, or overflows
/// seconds while applying the unit multiplier.
pub fn parse_duration(value: &str) -> Result<Duration, EnvParseError> {
    let value = value.trim();

    if value.is_empty() {
        return Err(EnvParseError::new(value, "a duration"));
    }

    let Some(unit_start) = value.find(|character: char| !character.is_ascii_digit()) else {
        return parse_duration_amount(value, value, "");
    };

    let amount = value[..unit_start].trim();
    let unit = value[unit_start..].trim().to_ascii_lowercase();

    parse_duration_amount(value, amount, &unit)
}

/// Parses an optional duration environment variable.
///
/// # Errors
///
/// Returns [`EnvParseError`] when the variable is set to a non-empty value that cannot be parsed as
/// a duration.
pub fn duration_var(name: &str) -> Result<Option<Duration>, EnvParseError> {
    var(name)
        .map(|value| parse_duration(&value).map_err(|error| error.with_name(name)))
        .transpose()
}

/// Parses a duration environment variable or returns a fallback.
///
/// # Errors
///
/// Returns [`EnvParseError`] when the variable is set to a non-empty value that cannot be parsed as
/// a duration.
pub fn duration_var_or(name: &str, fallback: Duration) -> Result<Duration, EnvParseError> {
    duration_var(name).map(|value| value.unwrap_or(fallback))
}

fn normalize_string(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_owned())
}

fn parse_duration_amount(
    original: &str,
    amount: &str,
    unit: &str,
) -> Result<Duration, EnvParseError> {
    let amount = parse_u64(amount).map_err(|_| EnvParseError::new(original, "a duration"))?;

    match unit {
        "" | "s" | "sec" | "secs" | "second" | "seconds" => Ok(Duration::from_secs(amount)),
        "ms" | "millisecond" | "milliseconds" => Ok(Duration::from_millis(amount)),
        "m" | "min" | "mins" | "minute" | "minutes" => duration_from_seconds(amount, 60, original),
        "h" | "hr" | "hrs" | "hour" | "hours" => duration_from_seconds(amount, 60 * 60, original),
        "d" | "day" | "days" => duration_from_seconds(amount, 24 * 60 * 60, original),
        _ => Err(EnvParseError::new(original, "a duration")),
    }
}

fn duration_from_seconds(
    amount: u64,
    multiplier: u64,
    original: &str,
) -> Result<Duration, EnvParseError> {
    amount
        .checked_mul(multiplier)
        .map(Duration::from_secs)
        .ok_or_else(|| EnvParseError::new(original, "a duration"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const ABSENT_NAME: &str = "AWS_LAMBDA_POWERTOOLS_CORE_TEST_ABSENT_8E9A14D7";

    #[test]
    fn truthy_and_falsy_values_are_recognized() {
        for value in ["1", "true", "TRUE", " yes ", "Y", "on"] {
            assert!(is_truthy(value));
            assert_eq!(parse_bool(value), Ok(true));
        }

        for value in ["0", "false", "FALSE", " no ", "N", "off"] {
            assert!(is_falsy(value));
            assert_eq!(parse_bool(value), Ok(false));
        }
    }

    #[test]
    fn invalid_bool_returns_context() {
        let error = parse_bool("sometimes").expect_err("value should not parse");

        assert_eq!(error.value(), "sometimes");
        assert_eq!(error.expected(), "a boolean");
        assert_eq!(error.name(), None);
    }

    #[test]
    fn absent_bool_uses_false_or_fallback() {
        assert!(!bool_var(ABSENT_NAME));
        assert!(bool_var_or(ABSENT_NAME, true));
        assert_eq!(try_bool_var(ABSENT_NAME), Ok(None));
    }

    #[test]
    fn integers_trim_input_and_reject_invalid_values() {
        assert_eq!(parse_i64(" -42 "), Ok(-42));
        assert_eq!(parse_u64("42"), Ok(42));
        assert!(parse_u64("-1").is_err());
        assert!(parse_i64("").is_err());
        assert_eq!(u64_var_or(ABSENT_NAME, 30), Ok(30));
    }

    #[test]
    fn durations_support_seconds_and_units() {
        assert_eq!(parse_duration("15"), Ok(Duration::from_secs(15)));
        assert_eq!(parse_duration("250ms"), Ok(Duration::from_millis(250)));
        assert_eq!(parse_duration("2 minutes"), Ok(Duration::from_secs(120)));
        assert_eq!(parse_duration("3h"), Ok(Duration::from_secs(10_800)));
        assert_eq!(parse_duration("1 day"), Ok(Duration::from_secs(86_400)));
    }

    #[test]
    fn durations_reject_invalid_values() {
        assert!(parse_duration("-5s").is_err());
        assert!(parse_duration("5fortnights").is_err());
        assert!(parse_duration("seconds").is_err());
        assert!(parse_duration("").is_err());
    }
}
