//! Seekable streaming utility.

mod error;
mod source;
mod stream;
mod transform;

pub use error::{StreamingError, StreamingErrorKind, StreamingResult};
pub use source::{BytesRangeSource, RangeSource};
pub use stream::SeekableStream;

#[cfg(feature = "csv")]
pub use transform::{csv_reader, csv_reader_with_builder};

#[cfg(feature = "gzip")]
pub use transform::gzip_decoder;
