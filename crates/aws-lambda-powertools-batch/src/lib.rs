//! Batch processing utility.

mod failure;
mod processor;
mod record;
mod response;
#[cfg(feature = "aws-lambda-events")]
mod sqs;

pub use failure::BatchItemFailure;
pub use processor::{BatchProcessingReport, BatchProcessor, BatchRecordResult};
pub use record::BatchRecord;
pub use response::BatchResponse;
