//! Idempotency errors.

use std::fmt;

use crate::{IdempotencyKey, IdempotencyStoreError};

/// Result returned by idempotency workflow operations.
pub type IdempotencyResult<T> = Result<T, IdempotencyError>;

/// Error returned by idempotency workflow operations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IdempotencyError {
    /// The configured key source did not produce an idempotency key.
    MissingKey,
    /// Another operation with the same idempotency key is still in progress.
    AlreadyInProgress {
        /// Idempotency key for the in-progress operation.
        key: IdempotencyKey,
    },
    /// A stored payload hash does not match the current payload hash.
    PayloadMismatch {
        /// Idempotency key for the mismatched operation.
        key: IdempotencyKey,
    },
    /// A completed record does not contain response data to replay.
    MissingStoredResponse {
        /// Idempotency key for the incomplete completed record.
        key: IdempotencyKey,
    },
    /// The persistence store failed.
    Store {
        /// Store error message.
        message: String,
    },
    /// JSON serialization or deserialization failed.
    Serialization {
        /// Serialization error message.
        message: String,
    },
    /// Idempotency key extraction failed.
    KeyExtraction {
        /// Key extraction error message.
        message: String,
    },
}

impl IdempotencyError {
    /// Creates an idempotency store error.
    #[must_use]
    pub fn store(message: impl Into<String>) -> Self {
        Self::Store {
            message: message.into(),
        }
    }

    /// Creates a serialization error.
    #[must_use]
    pub fn serialization(message: impl Into<String>) -> Self {
        Self::Serialization {
            message: message.into(),
        }
    }

    /// Creates an idempotency key extraction error.
    #[must_use]
    pub fn key_extraction(message: impl Into<String>) -> Self {
        Self::KeyExtraction {
            message: message.into(),
        }
    }
}

impl fmt::Display for IdempotencyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingKey => formatter.write_str("idempotency key is missing"),
            Self::AlreadyInProgress { key } => {
                write!(formatter, "idempotency key {key} is already in progress")
            }
            Self::PayloadMismatch { key } => {
                write!(
                    formatter,
                    "payload hash does not match idempotency key {key}"
                )
            }
            Self::MissingStoredResponse { key } => {
                write!(
                    formatter,
                    "stored response is missing for idempotency key {key}"
                )
            }
            Self::Store { message }
            | Self::Serialization { message }
            | Self::KeyExtraction { message } => formatter.write_str(message),
        }
    }
}

impl std::error::Error for IdempotencyError {}

impl From<IdempotencyStoreError> for IdempotencyError {
    fn from(error: IdempotencyStoreError) -> Self {
        Self::store(error.to_string())
    }
}

/// Error returned when running an idempotent handler.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IdempotencyExecutionError<E> {
    /// Idempotency processing failed before or after handler execution.
    Idempotency(IdempotencyError),
    /// The wrapped handler returned an error.
    Handler(E),
}

impl<E> From<IdempotencyError> for IdempotencyExecutionError<E> {
    fn from(error: IdempotencyError) -> Self {
        Self::Idempotency(error)
    }
}

impl<E> fmt::Display for IdempotencyExecutionError<E>
where
    E: fmt::Display,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Idempotency(error) => error.fmt(formatter),
            Self::Handler(error) => error.fmt(formatter),
        }
    }
}

impl<E> std::error::Error for IdempotencyExecutionError<E>
where
    E: std::error::Error + 'static,
{
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Idempotency(error) => Some(error),
            Self::Handler(error) => Some(error),
        }
    }
}
