//! Batch processing utility.

#[cfg(feature = "aws-lambda-events")]
mod dynamodb;
mod failure;
#[cfg(feature = "aws-lambda-events")]
mod kinesis;
mod processor;
mod record;
mod response;
#[cfg(feature = "aws-lambda-events")]
mod sqs;

pub use failure::BatchItemFailure;
pub use processor::{BatchProcessingReport, BatchProcessor, BatchRecordResult};
pub use record::BatchRecord;
pub use response::BatchResponse;
