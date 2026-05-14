//! `DynamoDB` stream batch adapters.

use std::fmt::Display;

use aws_lambda_events::event::dynamodb::{Event as DynamoDbEvent, EventRecord as DynamoDbRecord};
#[cfg(feature = "parser")]
use serde::de::DeserializeOwned;

#[cfg(feature = "parser")]
use crate::ParsedBatchRecord;
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

    /// Parses `DynamoDB` stream `NewImage` records and processes only records
    /// that decode successfully.
    ///
    /// Records with missing or invalid `NewImage` items are reported as
    /// failures and are not passed to the handler.
    #[cfg(feature = "parser")]
    pub fn process_dynamodb_new_images<T, E>(
        &self,
        event: &DynamoDbEvent,
        handler: impl FnMut(&ParsedBatchRecord<'_, T, DynamoDbRecord>) -> Result<(), E>,
    ) -> BatchProcessingReport
    where
        T: DeserializeOwned,
        E: Display,
    {
        process_dynamodb_images(
            event,
            "NewImage",
            |record| &record.change.new_image,
            handler,
        )
    }

    /// Parses `DynamoDB` stream `NewImage` records and returns an AWS Lambda
    /// partial batch response.
    #[cfg(feature = "parser")]
    pub fn process_dynamodb_new_images_response<T, E>(
        &self,
        event: &DynamoDbEvent,
        handler: impl FnMut(&ParsedBatchRecord<'_, T, DynamoDbRecord>) -> Result<(), E>,
    ) -> BatchResponse
    where
        T: DeserializeOwned,
        E: Display,
    {
        self.process_dynamodb_new_images(event, handler).response()
    }

    /// Parses `DynamoDB` stream `OldImage` records and processes only records
    /// that decode successfully.
    ///
    /// Records with missing or invalid `OldImage` items are reported as
    /// failures and are not passed to the handler.
    #[cfg(feature = "parser")]
    pub fn process_dynamodb_old_images<T, E>(
        &self,
        event: &DynamoDbEvent,
        handler: impl FnMut(&ParsedBatchRecord<'_, T, DynamoDbRecord>) -> Result<(), E>,
    ) -> BatchProcessingReport
    where
        T: DeserializeOwned,
        E: Display,
    {
        process_dynamodb_images(
            event,
            "OldImage",
            |record| &record.change.old_image,
            handler,
        )
    }

    /// Parses `DynamoDB` stream `OldImage` records and returns an AWS Lambda
    /// partial batch response.
    #[cfg(feature = "parser")]
    pub fn process_dynamodb_old_images_response<T, E>(
        &self,
        event: &DynamoDbEvent,
        handler: impl FnMut(&ParsedBatchRecord<'_, T, DynamoDbRecord>) -> Result<(), E>,
    ) -> BatchResponse
    where
        T: DeserializeOwned,
        E: Display,
    {
        self.process_dynamodb_old_images(event, handler).response()
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

#[cfg(feature = "parser")]
fn process_dynamodb_images<T, E>(
    event: &DynamoDbEvent,
    image_name: &'static str,
    select_image: impl Fn(&DynamoDbRecord) -> &serde_dynamo::Item,
    mut handler: impl FnMut(&ParsedBatchRecord<'_, T, DynamoDbRecord>) -> Result<(), E>,
) -> BatchProcessingReport
where
    T: DeserializeOwned,
    E: Display,
{
    let results = event.records.iter().enumerate().map(|(index, record)| {
        let item_identifier = dynamodb_item_identifier(index, record);
        let parsed = match parse_dynamodb_image(image_name, index, select_image(record)) {
            Ok(parsed) => parsed,
            Err(error) => {
                return BatchRecordResult::failure(item_identifier, error);
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

#[cfg(feature = "parser")]
fn parse_dynamodb_image<T>(
    image_name: &str,
    index: usize,
    image: &serde_dynamo::Item,
) -> Result<T, String>
where
    T: DeserializeOwned,
{
    if image.is_empty() {
        return Err(format!(
            "DynamoDB record at index {index} is missing {image_name}"
        ));
    }

    serde_dynamo::from_item(image.clone()).map_err(|error| {
        format!("DynamoDB record at index {index} {image_name} cannot be decoded: {error}")
    })
}

#[cfg(test)]
mod tests {
    use aws_lambda_events::event::dynamodb::{
        Event as DynamoDbEvent, EventRecord as DynamoDbRecord,
    };
    #[cfg(feature = "parser")]
    use serde::{Deserialize, Serialize};
    use serde_json::json;

    use crate::{BatchProcessor, BatchRecordResult};

    fn record(sequence_number: Option<&str>, event_id: &str) -> DynamoDbRecord {
        let mut record = DynamoDbRecord::default();
        record.event_id = event_id.to_owned();
        record.change.sequence_number = sequence_number.map(str::to_owned);
        record
    }

    #[cfg(feature = "parser")]
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    struct OrderEvent {
        order_id: String,
        quantity: u32,
    }

    #[cfg(feature = "parser")]
    #[derive(Debug, Serialize)]
    struct InvalidOrderEvent {
        order_id: String,
        quantity: String,
    }

    #[cfg(feature = "parser")]
    fn order_image(order_id: &str, quantity: u32) -> serde_dynamo::Item {
        serde_dynamo::to_item(OrderEvent {
            order_id: order_id.to_owned(),
            quantity,
        })
        .expect("order image serializes")
    }

    #[cfg(feature = "parser")]
    fn invalid_order_image(order_id: &str) -> serde_dynamo::Item {
        serde_dynamo::to_item(InvalidOrderEvent {
            order_id: order_id.to_owned(),
            quantity: "many".to_owned(),
        })
        .expect("invalid order image serializes")
    }

    #[cfg(feature = "parser")]
    fn record_with_new_image(
        sequence_number: &str,
        event_id: &str,
        image: serde_dynamo::Item,
    ) -> DynamoDbRecord {
        let mut record = record(Some(sequence_number), event_id);
        record.change.new_image = image;
        record
    }

    #[cfg(feature = "parser")]
    fn record_with_old_image(
        sequence_number: &str,
        event_id: &str,
        image: serde_dynamo::Item,
    ) -> DynamoDbRecord {
        let mut record = record(Some(sequence_number), event_id);
        record.change.old_image = image;
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

    #[cfg(feature = "parser")]
    #[test]
    fn process_dynamodb_new_images_marks_parse_failures_before_handler() {
        let mut event = DynamoDbEvent::default();
        event.records = vec![
            record_with_new_image("sequence-1", "event-1", order_image("order-1", 2)),
            record_with_new_image("sequence-2", "event-2", invalid_order_image("order-2")),
            record_with_new_image("sequence-3", "event-3", order_image("order-3", 4)),
        ];
        let mut handled = Vec::new();

        let report =
            BatchProcessor::new().process_dynamodb_new_images::<OrderEvent, _>(&event, |record| {
                handled.push(record.payload().order_id.clone());
                Ok::<(), &str>(())
            });

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
                .contains("DynamoDB record at index 1 NewImage cannot be decoded")
        );
        assert_eq!(report.stream_checkpoint(), Some("sequence-2"));
    }

    #[cfg(feature = "parser")]
    #[test]
    fn process_dynamodb_old_images_reports_missing_images_before_handler() {
        let mut event = DynamoDbEvent::default();
        event.records = vec![
            record_with_old_image("sequence-1", "event-1", order_image("order-1", 2)),
            record(Some("sequence-2"), "event-2"),
        ];
        let mut handled = Vec::new();

        let report =
            BatchProcessor::new().process_dynamodb_old_images::<OrderEvent, _>(&event, |record| {
                handled.push(record.payload().order_id.clone());
                Ok::<(), &str>(())
            });

        assert_eq!(handled, ["order-1"]);
        assert_eq!(
            report.results(),
            &[
                BatchRecordResult::success("sequence-1"),
                BatchRecordResult::failure(
                    "sequence-2",
                    "DynamoDB record at index 1 is missing OldImage"
                ),
            ]
        );
        assert_eq!(report.stream_checkpoint(), Some("sequence-2"));
    }
}
