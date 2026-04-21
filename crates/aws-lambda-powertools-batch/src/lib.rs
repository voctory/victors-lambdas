//! Batch processing utility.

mod failure;
mod processor;
mod record;
mod response;

pub use failure::BatchItemFailure;
pub use processor::{BatchProcessingReport, BatchProcessor, BatchRecordResult};
pub use record::BatchRecord;
pub use response::BatchResponse;
