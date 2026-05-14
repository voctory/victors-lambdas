//! Kinesis batch adapters.

use std::fmt::Display;

use aws_lambda_events::event::kinesis::{KinesisEvent, KinesisEventRecord};
#[cfg(feature = "parser")]
use serde::de::DeserializeOwned;
#[cfg(feature = "parser")]
use victors_lambdas_parser::EventParser;

#[cfg(feature = "parser")]
use crate::ParsedBatchRecord;
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

    /// Parses Kinesis record data and processes only records that decode successfully.
    ///
    /// Records with invalid JSON data are reported as failures and are not passed
    /// to the handler.
    #[cfg(feature = "parser")]
    pub fn process_kinesis_records<T, E>(
        &self,
        event: &KinesisEvent,
        parser: &EventParser,
        mut handler: impl FnMut(&ParsedBatchRecord<'_, T, KinesisEventRecord>) -> Result<(), E>,
    ) -> BatchProcessingReport
    where
        T: DeserializeOwned,
        E: Display,
    {
        let results = event.records.iter().enumerate().map(|(index, record)| {
            let item_identifier = kinesis_item_identifier(index, record);
            let parsed = match parser.parse_json_slice::<T>(&record.kinesis.data) {
                Ok(parsed) => parsed.into_payload(),
                Err(error) => {
                    return BatchRecordResult::failure(item_identifier, error.to_string());
                }
            };
            let parsed_record = ParsedBatchRecord::new(item_identifier.clone(), parsed, record);

            match handler(&parsed_record) {
                Ok(()) => BatchRecordResult::success(item_identifier),
                Err(error) => BatchRecordResult::failure(item_identifier, error.to_string()),
            }
        });

        BatchProcessingReport::from_results(results)
    }

    /// Parses Kinesis record data and returns an AWS Lambda partial batch response.
    #[cfg(feature = "parser")]
    pub fn process_kinesis_records_response<T, E>(
        &self,
        event: &KinesisEvent,
        parser: &EventParser,
        handler: impl FnMut(&ParsedBatchRecord<'_, T, KinesisEventRecord>) -> Result<(), E>,
    ) -> BatchResponse
    where
        T: DeserializeOwned,
        E: Display,
    {
        self.process_kinesis_records(event, parser, handler)
            .response()
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
    #[cfg(feature = "parser")]
    use serde::Deserialize;
    use serde_json::json;
    #[cfg(feature = "parser")]
    use victors_lambdas_parser::EventParser;

    use crate::{BatchProcessor, BatchRecordResult};

    fn record(sequence_number: &str, event_id: &str) -> KinesisEventRecord {
        let mut record = KinesisEventRecord::default();
        record.event_id = Some(event_id.to_owned());
        record.kinesis.sequence_number = sequence_number.to_owned();
        record
    }

    #[cfg(feature = "parser")]
    fn data_record(sequence_number: &str, event_id: &str, data: &[u8]) -> KinesisEventRecord {
        let mut record = record(sequence_number, event_id);
        record.kinesis.data.extend_from_slice(data);
        record
    }

    #[cfg(feature = "parser")]
    #[derive(Debug, Deserialize, Eq, PartialEq)]
    struct OrderEvent {
        order_id: String,
        quantity: u32,
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

    #[cfg(feature = "parser")]
    #[test]
    fn process_kinesis_records_marks_parse_failures_before_handler() {
        let mut event = KinesisEvent::default();
        event.records = vec![
            data_record(
                "sequence-1",
                "event-1",
                br#"{"order_id":"order-1","quantity":2}"#,
            ),
            data_record(
                "sequence-2",
                "event-2",
                br#"{"order_id":"order-2","quantity":"many"}"#,
            ),
            data_record(
                "sequence-3",
                "event-3",
                br#"{"order_id":"order-3","quantity":4}"#,
            ),
        ];
        let mut handled = Vec::new();

        let report = BatchProcessor::new().process_kinesis_records::<OrderEvent, _>(
            &event,
            &EventParser::new(),
            |record| {
                handled.push(record.payload().order_id.clone());
                Ok::<(), &str>(())
            },
        );

        assert_eq!(handled, ["order-1", "order-3"]);
        assert_eq!(
            report.results()[0],
            BatchRecordResult::success("sequence-1")
        );
        assert_eq!(
            report.results()[2],
            BatchRecordResult::success("sequence-3")
        );
        assert_eq!(report.results()[1].item_identifier(), "sequence-2");
        assert!(report.results()[1].is_failure());
        assert!(
            report.results()[1]
                .error()
                .expect("parse failure records an error")
                .contains("invalid type: string \"many\", expected u32")
        );
        assert_eq!(report.stream_checkpoint(), Some("sequence-2"));
    }
}
