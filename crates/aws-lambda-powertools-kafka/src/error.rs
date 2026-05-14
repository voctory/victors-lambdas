//! Kafka consumer errors.

use std::{error::Error, fmt};

/// High-level Kafka consumer error category.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KafkaConsumerErrorKind {
    /// A base64-encoded field could not be decoded.
    Base64,
    /// Decoded bytes were not valid UTF-8.
    Utf8,
    /// A JSON field could not be decoded into the target type.
    Json,
    /// A header value could not be decoded.
    Header,
}

/// Error returned by Kafka consumer helpers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KafkaConsumerError {
    kind: KafkaConsumerErrorKind,
    message: String,
}

impl KafkaConsumerError {
    /// Creates a Kafka consumer error.
    #[must_use]
    pub fn new(kind: KafkaConsumerErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    /// Creates a base64 decode error.
    #[must_use]
    pub fn base64(field: &str, error: impl fmt::Display) -> Self {
        Self::new(
            KafkaConsumerErrorKind::Base64,
            format!("Kafka record {field} is not valid base64: {error}"),
        )
    }

    /// Creates a UTF-8 decode error.
    #[must_use]
    pub fn utf8(field: &str, error: impl fmt::Display) -> Self {
        Self::new(
            KafkaConsumerErrorKind::Utf8,
            format!("Kafka record {field} is not valid UTF-8: {error}"),
        )
    }

    /// Creates a JSON decode error.
    #[must_use]
    pub fn json(field: &str, error: impl fmt::Display) -> Self {
        Self::new(
            KafkaConsumerErrorKind::Json,
            format!("Kafka record {field} could not be decoded as JSON: {error}"),
        )
    }

    /// Creates a header decode error.
    #[must_use]
    pub fn header(header: &str, error: impl fmt::Display) -> Self {
        Self::new(
            KafkaConsumerErrorKind::Header,
            format!("Kafka record header {header:?} is not valid UTF-8: {error}"),
        )
    }

    /// Returns the error category.
    #[must_use]
    pub const fn kind(&self) -> KafkaConsumerErrorKind {
        self.kind
    }

    /// Returns the human-readable error message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for KafkaConsumerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for KafkaConsumerError {}

/// Result type returned by Kafka consumer helpers.
pub type KafkaConsumerResult<T> = Result<T, KafkaConsumerError>;
