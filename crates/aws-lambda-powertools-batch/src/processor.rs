//! Batch processing facade.

use crate::{BatchItemFailure, BatchRecord};

/// Processes batch records and builds partial failure responses.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BatchProcessor;

impl BatchProcessor {
    /// Creates a batch processor.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Returns an empty failure list for an all-success batch.
    #[must_use]
    pub fn all_success<T>(&self, _records: &[BatchRecord<T>]) -> Vec<BatchItemFailure> {
        Vec::new()
    }
}
