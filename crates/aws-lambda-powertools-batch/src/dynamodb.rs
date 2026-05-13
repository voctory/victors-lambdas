//! `DynamoDB` stream batch adapters.

use std::fmt::Display;

use aws_lambda_events::event::dynamodb::{Event as DynamoDbEvent, EventRecord as DynamoDbRecord};

use crate::{BatchProcessingReport, BatchProcessor, BatchRecordResult, BatchResponse};

impl BatchProcessor {
    /// Processes `DynamoDB` stream records and builds a batch processing report.
    ///
    /// Failed records are identified by their `DynamoDB` stream sequence number,
    /// matching AWS Lambda partial batch response semantics for stream event
    /// sources.
    pub fn process_dynamodb<E>(
        &self,
        event: &DynamoDbEvent,
        mut handler: impl FnMut(&DynamoDbRecord) -> Result<(), E>,
    ) -> BatchProcessingReport
    where
        E: Display,
    {
        let results = event.records.iter().enumerate().map(|(index, record)| {
            let item_identifier = dynamodb_item_identifier(index, record);
            match handler(record) {
                Ok(()) => BatchRecordResult::success(item_identifier),
                Err(error) => BatchRecordResult::failure(item_identifier, error.to_string()),
            }
        });

        BatchProcessingReport::from_results(results)
    }

    /// Processes `DynamoDB` stream records and returns an AWS Lambda partial batch response.
    pub fn process_dynamodb_response<E>(
        &self,
        event: &DynamoDbEvent,
        handler: impl FnMut(&DynamoDbRecord) -> Result<(), E>,
    ) -> BatchResponse
    where
        E: Display,
    {
        self.process_dynamodb(event, handler).response()
    }
}

fn dynamodb_item_identifier(index: usize, record: &DynamoDbRecord) -> String {
    record
        .change
        .sequence_number
        .clone()
        .filter(|sequence_number| !sequence_number.is_empty())
        .unwrap_or_else(|| {
            if record.event_id.is_empty() {
                format!("record-{index}")
            } else {
                record.event_id.clone()
            }
        })
}

#[cfg(test)]
mod tests {
    use aws_lambda_events::event::dynamodb::{
        Event as DynamoDbEvent, EventRecord as DynamoDbRecord,
    };
    use serde_json::json;

    use crate::{BatchProcessor, BatchRecordResult};

    fn record(sequence_number: Option<&str>, event_id: &str) -> DynamoDbRecord {
        let mut record = DynamoDbRecord::default();
        record.event_id = event_id.to_owned();
        record.change.sequence_number = sequence_number.map(str::to_owned);
        record
    }

    #[test]
    fn process_dynamodb_uses_sequence_number_failures() {
        let mut event = DynamoDbEvent::default();
        event.records = vec![
            record(Some("sequence-1"), "event-1"),
            record(Some("sequence-2"), "event-2"),
        ];

        let report = BatchProcessor::new().process_dynamodb(&event, |record| {
            if record.change.sequence_number.as_deref() == Some("sequence-2") {
                Err("handler failed")
            } else {
                Ok(())
            }
        });

        assert_eq!(
            report.results(),
            &[
                BatchRecordResult::success("sequence-1"),
                BatchRecordResult::failure("sequence-2", "handler failed"),
            ]
        );
        assert_eq!(
            serde_json::to_value(report.response()).expect("response serializes"),
            json!({
                "batchItemFailures": [
                    {
                        "itemIdentifier": "sequence-2",
                    },
                ],
            })
        );
    }

    #[test]
    fn process_dynamodb_falls_back_to_event_id_when_sequence_number_is_missing() {
        let mut event = DynamoDbEvent::default();
        event.records = vec![record(None, "event-1")];

        let report = BatchProcessor::new().process_dynamodb(&event, |_record| Err("failed"));

        assert_eq!(
            report.results(),
            &[BatchRecordResult::failure("event-1", "failed")]
        );
    }
}
