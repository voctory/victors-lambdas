//! Streaming errors.

use std::{error::Error, fmt, io};

/// High-level streaming error category.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StreamingErrorKind {
    /// A range source returned an I/O error.
    Io,
    /// A seek request would move before the start of the stream.
    InvalidSeek,
}

/// Error returned by streaming helpers.
#[derive(Debug)]
pub struct StreamingError {
    kind: StreamingErrorKind,
    message: String,
    source: Option<Box<dyn Error + Send + Sync>>,
}

impl StreamingError {
    /// Creates a streaming error.
    #[must_use]
    pub fn new(kind: StreamingErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            source: None,
        }
    }

    /// Creates an I/O error.
    #[must_use]
    pub fn io(error: io::Error) -> Self {
        Self {
            kind: StreamingErrorKind::Io,
            message: format!("streaming source I/O error: {error}"),
            source: Some(Box::new(error)),
        }
    }

    /// Creates an invalid seek error.
    #[must_use]
    pub fn invalid_seek(position: i128) -> Self {
        Self::new(
            StreamingErrorKind::InvalidSeek,
            format!("streaming seek target {position} is before the start of the stream"),
        )
    }

    /// Returns the error category.
    #[must_use]
    pub const fn kind(&self) -> StreamingErrorKind {
        self.kind
    }

    /// Returns the human-readable error message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    pub(crate) fn into_io_error(self) -> io::Error {
        match self.kind {
            StreamingErrorKind::Io => io::Error::other(self),
            StreamingErrorKind::InvalidSeek => io::Error::new(io::ErrorKind::InvalidInput, self),
        }
    }
}

impl fmt::Display for StreamingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for StreamingError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.source
            .as_ref()
            .map(|source| source.as_ref() as &(dyn Error + 'static))
    }
}

impl From<io::Error> for StreamingError {
    fn from(error: io::Error) -> Self {
        Self::io(error)
    }
}

/// Result type returned by streaming helpers.
pub type StreamingResult<T> = Result<T, StreamingError>;
