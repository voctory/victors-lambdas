//! Data masking errors.

use std::{error::Error, fmt};

/// High-level data masking error category.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DataMaskingErrorKind {
    /// Input JSON could not be parsed.
    Json,
    /// A field path was empty or malformed.
    InvalidPath,
    /// A requested field path did not match the payload.
    MissingField,
    /// A regex masking expression could not be compiled.
    Regex,
}

/// Error returned by data masking helpers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DataMaskingError {
    kind: DataMaskingErrorKind,
    message: String,
}

impl DataMaskingError {
    /// Creates a data masking error.
    #[must_use]
    pub fn new(kind: DataMaskingErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    /// Creates a JSON parsing error.
    #[must_use]
    pub fn json(error: impl fmt::Display) -> Self {
        Self::new(
            DataMaskingErrorKind::Json,
            format!("data masking input is not valid JSON: {error}"),
        )
    }

    /// Creates an invalid path error.
    #[must_use]
    pub fn invalid_path(path: &str) -> Self {
        Self::new(
            DataMaskingErrorKind::InvalidPath,
            format!("data masking field path {path:?} is empty or malformed"),
        )
    }

    /// Creates a missing field error.
    #[must_use]
    pub fn missing_field(path: &str) -> Self {
        Self::new(
            DataMaskingErrorKind::MissingField,
            format!("data masking field path {path:?} did not match the payload"),
        )
    }

    /// Creates a regex compilation error.
    #[must_use]
    pub fn regex(pattern: &str, error: impl fmt::Display) -> Self {
        Self::new(
            DataMaskingErrorKind::Regex,
            format!("data masking regex pattern {pattern:?} could not be compiled: {error}"),
        )
    }

    /// Returns the error category.
    #[must_use]
    pub const fn kind(&self) -> DataMaskingErrorKind {
        self.kind
    }

    /// Returns the human-readable error message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for DataMaskingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for DataMaskingError {}

/// Result type returned by data masking helpers.
pub type DataMaskingResult<T> = Result<T, DataMaskingError>;
