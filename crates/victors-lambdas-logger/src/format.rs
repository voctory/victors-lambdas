//! Log formatting and redaction extension points.

use crate::LogValue;

/// Formats a structured log value into an emitted log line.
pub trait LogFormatter {
    /// Formats a structured log value.
    fn format(&self, value: &LogValue) -> String;
}

impl<F> LogFormatter for F
where
    F: Fn(&LogValue) -> String,
{
    fn format(&self, value: &LogValue) -> String {
        self(value)
    }
}

/// Default JSON log formatter.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct JsonLogFormatter;

impl LogFormatter for JsonLogFormatter {
    fn format(&self, value: &LogValue) -> String {
        value.to_json_string()
    }
}

/// Mutates a structured log value before formatting.
pub trait LogRedactor {
    /// Applies custom redaction or transformation.
    fn redact(&self, value: &mut LogValue);
}

impl<F> LogRedactor for F
where
    F: Fn(&mut LogValue),
{
    fn redact(&self, value: &mut LogValue) {
        self(value);
    }
}
