//! Kinesis batch adapters.

use std::fmt::Display;

use aws_lambda_events::event::kinesis::{KinesisEvent, KinesisEventRecord};

use crate::{BatchProcessingReport, BatchProcessor, BatchRecordResult, BatchResponse};

impl BatchProcessor {
    /// Processes Kinesis records and builds a batch processing report.
    ///
    /// Failed records are identified by their Kinesis sequence number, matching
    /// AWS Lambda partial batch response semantics for stream event sources.
    pub fn process_kinesis<E>(
        &self,
        event: &KinesisEvent,
        mut handler: impl FnMut(&KinesisEventRecord) -> Result<(), E>,
    ) -> BatchProcessingReport
    where
        E: Display,
    {
        let results = event.records.iter().enumerate().map(|(index, record)| {
            let item_identifier = kinesis_item_identifier(index, record);
            match handler(record) {
                Ok(()) => BatchRecordResult::success(item_identifier),
                Err(error) => BatchRecordResult::failure(item_identifier, error.to_string()),
            }
        });

        BatchProcessingReport::from_results(results)
    }

    /// Processes Kinesis records and returns an AWS Lambda partial batch response.
    pub fn process_kinesis_response<E>(
        &self,
        event: &KinesisEvent,
        handler: impl FnMut(&KinesisEventRecord) -> Result<(), E>,
    ) -> BatchResponse
    where
        E: Display,
    {
        self.process_kinesis(event, handler).response()
    }
}

fn kinesis_item_identifier(index: usize, record: &KinesisEventRecord) -> String {
    if record.kinesis.sequence_number.is_empty() {
        record
            .event_id
            .clone()
            .unwrap_or_else(|| format!("record-{index}"))
    } else {
        record.kinesis.sequence_number.clone()
    }
}

#[cfg(test)]
mod tests {
    use aws_lambda_events::event::kinesis::{KinesisEvent, KinesisEventRecord};
    use serde_json::json;

    use crate::{BatchProcessor, BatchRecordResult};

    fn record(sequence_number: &str, event_id: &str) -> KinesisEventRecord {
        let mut record = KinesisEventRecord::default();
        record.event_id = Some(event_id.to_owned());
        record.kinesis.sequence_number = sequence_number.to_owned();
        record
    }

    #[test]
    fn process_kinesis_uses_sequence_number_failures() {
        let mut event = KinesisEvent::default();
        event.records = vec![
            record("sequence-1", "event-1"),
            record("sequence-2", "event-2"),
        ];

        let report = BatchProcessor::new().process_kinesis(&event, |record| {
            if record.kinesis.sequence_number == "sequence-2" {
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
    fn process_kinesis_falls_back_to_event_id_when_sequence_number_is_missing() {
        let mut event = KinesisEvent::default();
        event.records = vec![record("", "event-1")];

        let report = BatchProcessor::new().process_kinesis(&event, |_record| Err("failed"));

        assert_eq!(
            report.results(),
            &[BatchRecordResult::failure("event-1", "failed")]
        );
    }
}
