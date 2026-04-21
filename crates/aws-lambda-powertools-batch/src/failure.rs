//! Batch item failures.

use serde::{Deserialize, Serialize};

/// Identifies a failed batch item for partial batch responses.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BatchItemFailure {
    #[serde(rename = "itemIdentifier")]
    item_identifier: String,
}

impl BatchItemFailure {
    /// Creates a batch item failure.
    #[must_use]
    pub fn new(item_identifier: impl Into<String>) -> Self {
        Self {
            item_identifier: item_identifier.into(),
        }
    }

    /// Returns the failed item identifier.
    #[must_use]
    pub fn item_identifier(&self) -> &str {
        &self.item_identifier
    }
}
