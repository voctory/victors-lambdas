//! `JMESPath` utility errors.

use std::{error::Error, fmt};

/// High-level `JMESPath` error category.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JmespathErrorKind {
    /// The `JMESPath` expression could not be compiled.
    Compile,
    /// The expression failed while searching input data.
    Search,
    /// The selected value could not be converted to the requested Rust type.
    Decode,
    /// The selected `JMESPath` value could not be converted to JSON.
    Encode,
}

/// Error returned by `JMESPath` extraction helpers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JmespathError {
    kind: JmespathErrorKind,
    message: String,
}

impl JmespathError {
    /// Creates a `JMESPath` utility error.
    #[must_use]
    pub fn new(kind: JmespathErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    /// Creates an expression compile error.
    #[must_use]
    pub fn compile(expression: &str, error: impl fmt::Display) -> Self {
        Self::new(
            JmespathErrorKind::Compile,
            format!("failed to compile JMESPath expression {expression:?}: {error}"),
        )
    }

    /// Creates a search error.
    #[must_use]
    pub fn search(expression: &str, error: impl fmt::Display) -> Self {
        Self::new(
            JmespathErrorKind::Search,
            format!("failed to evaluate JMESPath expression {expression:?}: {error}"),
        )
    }

    /// Creates a decode error.
    #[must_use]
    pub fn decode(error: impl fmt::Display) -> Self {
        Self::new(
            JmespathErrorKind::Decode,
            format!("failed to decode JMESPath result: {error}"),
        )
    }

    /// Creates an encode error.
    #[must_use]
    pub fn encode(error: impl fmt::Display) -> Self {
        Self::new(
            JmespathErrorKind::Encode,
            format!("failed to encode JMESPath result as JSON: {error}"),
        )
    }

    /// Returns the error category.
    #[must_use]
    pub const fn kind(&self) -> JmespathErrorKind {
        self.kind
    }

    /// Returns the human-readable error message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for JmespathError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for JmespathError {}

/// Result type returned by `JMESPath` extraction helpers.
pub type JmespathResult<T> = Result<T, JmespathError>;
