//! Partial batch responses.

use serde::{Deserialize, Serialize};

use crate::BatchItemFailure;

/// AWS Lambda partial batch response payload.
///
/// Serializes as `{"batchItemFailures":[{"itemIdentifier":"..."}]}`.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct BatchResponse {
    #[serde(rename = "batchItemFailures")]
    batch_item_failures: Vec<BatchItemFailure>,
}

impl BatchResponse {
    /// Creates an empty partial batch response.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            batch_item_failures: Vec::new(),
        }
    }

    /// Creates a partial batch response from failed item identifiers.
    #[must_use]
    pub fn from_failures(batch_item_failures: impl IntoIterator<Item = BatchItemFailure>) -> Self {
        Self {
            batch_item_failures: batch_item_failures.into_iter().collect(),
        }
    }

    /// Returns the failed item identifiers.
    #[must_use]
    pub fn batch_item_failures(&self) -> &[BatchItemFailure] {
        &self.batch_item_failures
    }

    /// Returns true when the response contains no failed item identifiers.
    #[must_use]
    pub fn is_success(&self) -> bool {
        self.batch_item_failures.is_empty()
    }
}
