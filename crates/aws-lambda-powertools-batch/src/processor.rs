//! Batch processing facade.

use std::fmt::Display;

use crate::{BatchItemFailure, BatchRecord, BatchResponse};

/// Processing result for a single batch record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BatchRecordResult {
    /// The record was processed successfully.
    Success {
        /// Identifier of the processed record.
        item_identifier: String,
    },
    /// The record failed and should be included in a partial batch response.
    Failure {
        /// Identifier of the failed record.
        item_identifier: String,
        /// Error message captured from the record handler.
        error: String,
    },
}

impl BatchRecordResult {
    /// Creates a successful record result.
    #[must_use]
    pub fn success(item_identifier: impl Into<String>) -> Self {
        Self::Success {
            item_identifier: item_identifier.into(),
        }
    }

    /// Creates a failed record result.
    #[must_use]
    pub fn failure(item_identifier: impl Into<String>, error: impl Into<String>) -> Self {
        Self::Failure {
            item_identifier: item_identifier.into(),
            error: error.into(),
        }
    }

    /// Returns the processed item identifier.
    #[must_use]
    pub fn item_identifier(&self) -> &str {
        match self {
            Self::Success { item_identifier }
            | Self::Failure {
                item_identifier, ..
            } => item_identifier,
        }
    }

    /// Returns true when the record processed successfully.
    #[must_use]
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success { .. })
    }

    /// Returns true when the record failed.
    #[must_use]
    pub fn is_failure(&self) -> bool {
        matches!(self, Self::Failure { .. })
    }

    /// Returns the captured error message for failed records.
    #[must_use]
    pub fn error(&self) -> Option<&str> {
        match self {
            Self::Success { .. } => None,
            Self::Failure { error, .. } => Some(error),
        }
    }

    fn as_failure(&self) -> Option<BatchItemFailure> {
        self.is_failure()
            .then(|| BatchItemFailure::new(self.item_identifier()))
    }
}

/// Processing report for all records in a batch.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BatchProcessingReport {
    results: Vec<BatchRecordResult>,
}

impl BatchProcessingReport {
    /// Creates an empty processing report.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            results: Vec::new(),
        }
    }

    /// Creates a processing report from record results.
    #[must_use]
    pub fn from_results(results: impl IntoIterator<Item = BatchRecordResult>) -> Self {
        Self {
            results: results.into_iter().collect(),
        }
    }

    /// Returns every record processing result in batch order.
    #[must_use]
    pub fn results(&self) -> &[BatchRecordResult] {
        &self.results
    }

    /// Returns the number of processed records.
    #[must_use]
    pub fn len(&self) -> usize {
        self.results.len()
    }

    /// Returns true when the report contains no record results.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.results.is_empty()
    }

    /// Returns true when every processed record succeeded.
    #[must_use]
    pub fn is_success(&self) -> bool {
        self.results.iter().all(BatchRecordResult::is_success)
    }

    /// Returns failed item identifiers for the batch.
    #[must_use]
    pub fn failures(&self) -> Vec<BatchItemFailure> {
        self.results
            .iter()
            .filter_map(BatchRecordResult::as_failure)
            .collect()
    }

    /// Returns an AWS Lambda partial batch response for failed records.
    #[must_use]
    pub fn response(&self) -> BatchResponse {
        BatchResponse::from_failures(self.failures())
    }
}

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

    /// Returns an empty AWS Lambda partial batch response for an all-success batch.
    #[must_use]
    pub fn all_success_response<T>(&self, _records: &[BatchRecord<T>]) -> BatchResponse {
        BatchResponse::new()
    }

    /// Processes batch records with a handler and records each result.
    ///
    /// The returned report preserves input order and can build an AWS Lambda
    /// partial batch response containing only failed item identifiers.
    pub fn process<T, E>(
        &self,
        records: &[BatchRecord<T>],
        mut handler: impl FnMut(&BatchRecord<T>) -> Result<(), E>,
    ) -> BatchProcessingReport
    where
        E: Display,
    {
        let results = records.iter().map(|record| match handler(record) {
            Ok(()) => BatchRecordResult::success(record.item_identifier()),
            Err(error) => BatchRecordResult::failure(record.item_identifier(), error.to_string()),
        });

        BatchProcessingReport::from_results(results)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{BatchProcessor, BatchRecordResult};
    use crate::{BatchRecord, BatchResponse};

    #[test]
    fn processing_report_records_successes_and_failures() {
        let records = [
            BatchRecord::new("message-1", 1),
            BatchRecord::new("message-2", 2),
            BatchRecord::new("message-3", 3),
        ];

        let report = BatchProcessor::new().process(&records, |record| {
            if *record.payload() == 2 {
                Err("handler failed")
            } else {
                Ok(())
            }
        });

        assert_eq!(report.len(), 3);
        assert!(!report.is_success());
        assert_eq!(
            report.results(),
            &[
                BatchRecordResult::success("message-1"),
                BatchRecordResult::failure("message-2", "handler failed"),
                BatchRecordResult::success("message-3"),
            ]
        );
        assert_eq!(report.results()[1].error(), Some("handler failed"));
    }

    #[test]
    fn report_builds_aws_partial_batch_response() {
        let report = super::BatchProcessingReport::from_results([
            BatchRecordResult::success("message-1"),
            BatchRecordResult::failure("message-2", "handler failed"),
        ]);

        let response = report.response();
        let serialized = serde_json::to_value(&response).expect("response serializes");

        assert_eq!(
            serialized,
            json!({
                "batchItemFailures": [
                    {
                        "itemIdentifier": "message-2",
                    },
                ],
            })
        );
        assert_eq!(
            response.batch_item_failures()[0].item_identifier(),
            "message-2"
        );
    }

    #[test]
    fn empty_response_serializes_with_aws_field_name() {
        let response = BatchResponse::new();

        assert!(response.is_success());
        assert_eq!(
            serde_json::to_value(response).expect("response serializes"),
            json!({ "batchItemFailures": [] })
        );
    }
}
