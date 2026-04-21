//! Event parsing utility.

mod error;
mod parser;

pub use error::{ParseError, ParseErrorKind};
pub use parser::{EventParser, ParsedEvent};
