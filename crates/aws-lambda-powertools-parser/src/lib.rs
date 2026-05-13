//! Event parsing utility.

#[cfg(feature = "aws-lambda-events")]
mod envelope;
mod error;
mod parser;
mod transfer_family;

pub use error::{ParseError, ParseErrorKind};
pub use parser::{EventParser, ParsedEvent};
pub use transfer_family::{
    TransferFamilyAuthorizerEvent, TransferFamilyAuthorizerResponse,
    TransferFamilyHomeDirectoryEntry, TransferFamilyHomeDirectoryType, TransferFamilyPosixProfile,
    TransferFamilyProtocol, TransferFamilyResponseError, TransferFamilyResponseResult,
};
