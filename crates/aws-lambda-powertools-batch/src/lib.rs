//! Batch processing utility.

mod failure;
mod processor;
mod record;

pub use failure::BatchItemFailure;
pub use processor::BatchProcessor;
pub use record::BatchRecord;
