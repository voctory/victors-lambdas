//! Seekable streaming utility.

mod error;
mod s3;
mod source;
mod stream;
mod transform;

pub use error::{StreamingError, StreamingErrorKind, StreamingResult};
pub use s3::{
    S3GetObjectRangeRequest, S3HeadObjectOutput, S3HeadObjectRequest, S3ObjectClient,
    S3ObjectIdentifier, S3RangeSource,
};
pub use source::{BytesRangeSource, RangeSource};
pub use stream::SeekableStream;

#[cfg(feature = "csv")]
pub use transform::{csv_reader, csv_reader_with_builder};

#[cfg(feature = "gzip")]
pub use transform::gzip_decoder;

#[cfg(feature = "zip")]
pub use transform::zip_archive;
