//! Kafka batch adapters.

use std::fmt::Display;

use aws_lambda_events::event::kafka::{KafkaEvent, KafkaRecord};
use serde::{Deserialize, Serialize};

use crate::BatchProcessor;

/// Identifies a failed Kafka batch item by topic-partition and offset.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct KafkaBatchItemIdentifier {
    partition: String,
    offset: i64,
}

impl KafkaBatchItemIdentifier {
    /// Creates a Kafka batch item identifier.
    #[must_use]
    pub fn new(partition: impl Into<String>, offset: i64) -> Self {
        Self {
            partition: partition.into(),
            offset,
        }
    }

    /// Returns the failed Kafka topic-partition identifier.
    #[must_use]
    pub fn partition(&self) -> &str {
        &self.partition
    }

    /// Returns the failed Kafka offset.
    #[must_use]
    pub const fn offset(&self) -> i64 {
        self.offset
    }
}

/// Identifies a failed Kafka batch item for partial batch responses.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct KafkaBatchItemFailure {
    #[serde(rename = "itemIdentifier")]
    item_identifier: KafkaBatchItemIdentifier,
}

impl KafkaBatchItemFailure {
    /// Creates a Kafka batch item failure.
    #[must_use]
    pub const fn new(item_identifier: KafkaBatchItemIdentifier) -> Self {
        Self { item_identifier }
    }

    /// Returns the failed Kafka batch item identifier.
    #[must_use]
    pub const fn item_identifier(&self) -> &KafkaBatchItemIdentifier {
        &self.item_identifier
    }
}

/// AWS Lambda Kafka partial batch response payload.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct KafkaBatchResponse {
    #[serde(rename = "batchItemFailures")]
    batch_item_failures: Vec<KafkaBatchItemFailure>,
}

impl KafkaBatchResponse {
    /// Creates an empty Kafka partial batch response.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            batch_item_failures: Vec::new(),
        }
    }

    /// Creates a Kafka partial batch response from failed item identifiers.
    #[must_use]
    pub fn from_failures(
        batch_item_failures: impl IntoIterator<Item = KafkaBatchItemFailure>,
    ) -> Self {
        Self {
            batch_item_failures: batch_item_failures.into_iter().collect(),
        }
    }

    /// Returns the failed Kafka item identifiers.
    #[must_use]
    pub fn batch_item_failures(&self) -> &[KafkaBatchItemFailure] {
        &self.batch_item_failures
    }

    /// Returns true when the response contains no failed Kafka item identifiers.
    #[must_use]
    pub fn is_success(&self) -> bool {
        self.batch_item_failures.is_empty()
    }
}

/// Processing result for a single Kafka batch record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum KafkaBatchRecordResult {
    /// The record was processed successfully.
    Success {
        /// Identifier of the processed Kafka record.
        item_identifier: KafkaBatchItemIdentifier,
    },
    /// The record failed and should be included in a partial batch response.
    Failure {
        /// Identifier of the failed Kafka record.
        item_identifier: KafkaBatchItemIdentifier,
        /// Error message captured from the record handler.
        error: String,
    },
}

impl KafkaBatchRecordResult {
    /// Creates a successful Kafka record result.
    #[must_use]
    pub const fn success(item_identifier: KafkaBatchItemIdentifier) -> Self {
        Self::Success { item_identifier }
    }

    /// Creates a failed Kafka record result.
    #[must_use]
    pub fn failure(item_identifier: KafkaBatchItemIdentifier, error: impl Into<String>) -> Self {
        Self::Failure {
            item_identifier,
            error: error.into(),
        }
    }

    /// Returns the processed Kafka item identifier.
    #[must_use]
    pub const fn item_identifier(&self) -> &KafkaBatchItemIdentifier {
        match self {
            Self::Success { item_identifier }
            | Self::Failure {
                item_identifier, ..
            } => item_identifier,
        }
    }

    /// Returns true when the record processed successfully.
    #[must_use]
    pub const fn is_success(&self) -> bool {
        matches!(self, Self::Success { .. })
    }

    /// Returns true when the record failed.
    #[must_use]
    pub const fn is_failure(&self) -> bool {
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

    fn as_failure(&self) -> Option<KafkaBatchItemFailure> {
        self.is_failure()
            .then(|| KafkaBatchItemFailure::new(self.item_identifier().clone()))
    }
}

/// Processing report for all records in a Kafka batch.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct KafkaBatchProcessingReport {
    results: Vec<KafkaBatchRecordResult>,
}

impl KafkaBatchProcessingReport {
    /// Creates an empty Kafka processing report.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            results: Vec::new(),
        }
    }

    /// Creates a Kafka processing report from record results.
    #[must_use]
    pub fn from_results(results: impl IntoIterator<Item = KafkaBatchRecordResult>) -> Self {
        Self {
            results: results.into_iter().collect(),
        }
    }

    /// Returns every Kafka record processing result in deterministic order.
    #[must_use]
    pub fn results(&self) -> &[KafkaBatchRecordResult] {
        &self.results
    }

    /// Returns the number of processed Kafka records.
    #[must_use]
    pub fn len(&self) -> usize {
        self.results.len()
    }

    /// Returns true when the report contains no Kafka record results.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.results.is_empty()
    }

    /// Returns true when every processed Kafka record succeeded.
    #[must_use]
    pub fn is_success(&self) -> bool {
        self.results.iter().all(KafkaBatchRecordResult::is_success)
    }

    /// Returns failed Kafka item identifiers for the batch.
    #[must_use]
    pub fn failures(&self) -> Vec<KafkaBatchItemFailure> {
        self.results
            .iter()
            .filter_map(KafkaBatchRecordResult::as_failure)
            .collect()
    }

    /// Returns an AWS Lambda Kafka partial batch response for failed records.
    #[must_use]
    pub fn response(&self) -> KafkaBatchResponse {
        KafkaBatchResponse::from_failures(self.failures())
    }
}

impl BatchProcessor {
    /// Processes Lambda Kafka records and builds a batch processing report.
    ///
    /// Failed records are identified by the topic-partition and offset shape
    /// expected by Lambda Kafka partial batch responses.
    pub fn process_kafka<E>(
        &self,
        event: &KafkaEvent,
        mut handler: impl FnMut(&KafkaRecord) -> Result<(), E>,
    ) -> KafkaBatchProcessingReport
    where
        E: Display,
    {
        let mut grouped_records: Vec<_> = event.records.iter().collect();
        grouped_records.sort_by(|(left, _), (right, _)| left.cmp(right));

        let mut results = Vec::new();
        for (source_key, records) in grouped_records {
            results.reserve(records.len());
            for record in records {
                let item_identifier = kafka_item_identifier(source_key, record);
                match handler(record) {
                    Ok(()) => results.push(KafkaBatchRecordResult::success(item_identifier)),
                    Err(error) => results.push(KafkaBatchRecordResult::failure(
                        item_identifier,
                        error.to_string(),
                    )),
                }
            }
        }

        KafkaBatchProcessingReport::from_results(results)
    }

    /// Processes Lambda Kafka records and returns a Kafka partial batch response.
    pub fn process_kafka_response<E>(
        &self,
        event: &KafkaEvent,
        handler: impl FnMut(&KafkaRecord) -> Result<(), E>,
    ) -> KafkaBatchResponse
    where
        E: Display,
    {
        self.process_kafka(event, handler).response()
    }
}

fn kafka_item_identifier(source_key: &str, record: &KafkaRecord) -> KafkaBatchItemIdentifier {
    let partition = record
        .topic
        .as_deref()
        .filter(|topic| !topic.is_empty())
        .map_or_else(
            || {
                if source_key.trim().is_empty() {
                    record.partition.to_string()
                } else {
                    source_key.to_owned()
                }
            },
            |topic| format!("{topic}-{}", record.partition),
        );

    KafkaBatchItemIdentifier::new(partition, record.offset)
}

#[cfg(test)]
mod tests {
    use aws_lambda_events::event::kafka::{KafkaEvent, KafkaRecord};
    use serde_json::json;

    use crate::{BatchProcessor, KafkaBatchRecordResult};

    fn record(topic: Option<&str>, partition: i64, offset: i64) -> KafkaRecord {
        let mut record = KafkaRecord::default();
        record.topic = topic.map(str::to_owned);
        record.partition = partition;
        record.offset = offset;
        record
    }

    #[test]
    fn process_kafka_uses_topic_partition_and_offset_failures() {
        let mut event = KafkaEvent::default();
        event.records.insert(
            "orders-0".to_owned(),
            vec![record(Some("orders"), 0, 10), record(Some("orders"), 0, 11)],
        );

        let report = BatchProcessor::new().process_kafka(&event, |record| {
            if record.offset == 11 {
                Err("handler failed")
            } else {
                Ok(())
            }
        });

        assert_eq!(report.len(), 2);
        assert!(!report.is_success());
        assert!(report.results()[0].is_success());
        assert_eq!(
            report.results()[1],
            KafkaBatchRecordResult::failure(
                crate::KafkaBatchItemIdentifier::new("orders-0", 11),
                "handler failed"
            )
        );
        assert_eq!(
            serde_json::to_value(report.response()).expect("response serializes"),
            json!({
                "batchItemFailures": [
                    {
                        "itemIdentifier": {
                            "partition": "orders-0",
                            "offset": 11,
                        },
                    },
                ],
            })
        );
    }

    #[test]
    fn process_kafka_falls_back_to_source_key_when_topic_is_missing() {
        let mut event = KafkaEvent::default();
        event
            .records
            .insert("orders-1".to_owned(), vec![record(None, 1, 22)]);

        let response =
            BatchProcessor::new().process_kafka_response(&event, |_record| Err("failed"));
        let failure = &response.batch_item_failures()[0];

        assert_eq!(failure.item_identifier().partition(), "orders-1");
        assert_eq!(failure.item_identifier().offset(), 22);
    }
}
