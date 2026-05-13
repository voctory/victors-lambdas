//! Event parsing utility.

#[cfg(feature = "aws-lambda-events")]
mod envelope;
mod error;
mod parser;

pub use error::{ParseError, ParseErrorKind};
pub use parser::{EventParser, ParsedEvent};
