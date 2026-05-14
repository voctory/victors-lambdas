//! Batch processing utility.

#[cfg(feature = "aws-lambda-events")]
mod dynamodb;
mod failure;
#[cfg(feature = "aws-lambda-events")]
mod kinesis;
#[cfg(feature = "parser")]
mod parser;
mod processor;
mod record;
mod response;
#[cfg(feature = "aws-lambda-events")]
mod sqs;

pub use failure::BatchItemFailure;
#[cfg(feature = "parser")]
pub use parser::ParsedBatchRecord;
pub use processor::{BatchProcessingReport, BatchProcessor, BatchRecordResult};
pub use record::BatchRecord;
pub use response::BatchResponse;
