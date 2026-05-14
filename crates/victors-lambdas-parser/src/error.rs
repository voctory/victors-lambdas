//! Parser errors.

use serde_json::error::Category;

/// High-level parse error category.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParseErrorKind {
    /// Parsed input had the right syntax but could not be decoded into the target type.
    Data,
    /// Parsed input ended before a complete value was available.
    Eof,
    /// Parsing failed while reading input.
    Io,
    /// Parsed input was not syntactically valid.
    Syntax,
}

/// Error returned when event parsing fails.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParseError {
    kind: ParseErrorKind,
    message: String,
    line: Option<usize>,
    column: Option<usize>,
}

impl ParseError {
    /// Creates a parser error without source location metadata.
    #[must_use]
    pub fn new(kind: ParseErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            line: None,
            column: None,
        }
    }

    pub(crate) fn from_json_error(error: &serde_json::Error) -> Self {
        let kind = match error.classify() {
            Category::Data => ParseErrorKind::Data,
            Category::Eof => ParseErrorKind::Eof,
            Category::Io => ParseErrorKind::Io,
            Category::Syntax => ParseErrorKind::Syntax,
        };
        let line = (error.line() != 0).then_some(error.line());
        let column = (error.column() != 0).then_some(error.column());

        Self {
            kind,
            message: error.to_string(),
            line,
            column,
        }
    }

    /// Returns the parse error category.
    #[must_use]
    pub const fn kind(&self) -> ParseErrorKind {
        self.kind
    }

    /// Returns the parse error message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Returns the one-based source line when available.
    #[must_use]
    pub const fn line(&self) -> Option<usize> {
        self.line
    }

    /// Returns the one-based source column when available.
    #[must_use]
    pub const fn column(&self) -> Option<usize> {
        self.column
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ParseError {}
