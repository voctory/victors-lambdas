//! Seekable streaming utility.

#[cfg(feature = "async")]
mod async_stream;
mod error;
mod s3;
mod source;
mod stream;
mod transform;

#[cfg(feature = "async")]
pub use async_stream::AsyncSeekableStream;
pub use error::{StreamingError, StreamingErrorKind, StreamingResult};
#[cfg(feature = "async")]
pub use s3::AsyncS3ObjectClient;
#[cfg(feature = "s3")]
pub use s3::{AwsSdkS3AsyncRangeReader, AwsSdkS3ObjectClient, AwsSdkS3RangeReader};
pub use s3::{
    S3GetObjectRangeRequest, S3HeadObjectOutput, S3HeadObjectRequest, S3Object, S3ObjectClient,
    S3ObjectIdentifier, S3RangeSource,
};
#[cfg(feature = "async")]
pub use source::{AsyncRangeFuture, AsyncRangeSource};
pub use source::{BytesRangeSource, RangeSource};
pub use stream::SeekableStream;

#[cfg(feature = "csv")]
pub use transform::{csv_reader, csv_reader_with_builder};

#[cfg(feature = "gzip")]
pub use transform::gzip_decoder;

#[cfg(feature = "zip")]
pub use transform::zip_archive;
