//! Batch snippet for documentation.

use std::error::Error;

use aws_lambda_events::event::{
    kinesis::{KinesisEvent, KinesisEventRecord},
    sqs::{SqsEvent, SqsMessage},
};
use aws_lambda_powertools::prelude::{BatchProcessor, BatchRecord, EventParser};
use serde::Deserialize;

#[derive(Debug, Deserialize, Eq, PartialEq)]
struct OrderEvent {
    order_id: String,
    quantity: u32,
}

fn sqs_message(id: &str, body: &str) -> SqsMessage {
    let mut message = SqsMessage::default();
    message.message_id = Some(id.to_owned());
    message.body = Some(body.to_owned());
    message
}

fn kinesis_record(sequence_number: &str, data: &[u8]) -> KinesisEventRecord {
    let mut record = KinesisEventRecord::default();
    sequence_number.clone_into(&mut record.kinesis.sequence_number);
    record.kinesis.data.extend_from_slice(data);
    record
}

fn main() -> Result<(), Box<dyn Error>> {
    let records = vec![
        BatchRecord::new("record-1", "ok"),
        BatchRecord::new("record-2", "fail"),
        BatchRecord::new("record-3", "ok"),
    ];

    let report = BatchProcessor::new().process(&records, |record| {
        if *record.payload() == "fail" {
            Err("handler failed")
        } else {
            Ok(())
        }
    });

    let response = report.response();
    assert!(!response.is_success());
    assert_eq!(
        serde_json::to_string(&response)?,
        r#"{"batchItemFailures":[{"itemIdentifier":"record-2"}]}"#
    );

    let mut sqs_event = SqsEvent::default();
    sqs_event.records = vec![
        sqs_message("message-1", "ok"),
        sqs_message("message-2", "fail"),
        sqs_message("message-3", "not processed"),
    ];

    let fifo_response = BatchProcessor::new().process_sqs_fifo_response(&sqs_event, |message| {
        if message.body.as_deref() == Some("fail") {
            Err("invalid record")
        } else {
            Ok(())
        }
    });

    let failures = fifo_response.batch_item_failures();
    assert_eq!(failures.len(), 2);
    assert_eq!(failures[0].item_identifier(), "message-2");
    assert_eq!(failures[1].item_identifier(), "message-3");

    let mut parsed_event = SqsEvent::default();
    parsed_event.records = vec![
        sqs_message("message-4", r#"{"order_id":"order-4","quantity":2}"#),
        sqs_message("message-5", r#"{"order_id":"order-5","quantity":"many"}"#),
    ];

    let parsed_response = BatchProcessor::new()
        .process_sqs_message_bodies_response::<OrderEvent, _>(
            &parsed_event,
            &EventParser::new(),
            |record| {
                assert_eq!(record.payload().order_id, "order-4");
                Ok::<(), &str>(())
            },
        );
    let parsed_failures = parsed_response.batch_item_failures();
    assert_eq!(parsed_failures.len(), 1);
    assert_eq!(parsed_failures[0].item_identifier(), "message-5");

    let mut kinesis_event = KinesisEvent::default();
    kinesis_event.records = vec![
        kinesis_record("sequence-1", br#"{"order_id":"order-6","quantity":1}"#),
        kinesis_record("sequence-2", br#"{"order_id":"order-7","quantity":"many"}"#),
    ];

    let kinesis_report = BatchProcessor::new().process_kinesis_records::<OrderEvent, _>(
        &kinesis_event,
        &EventParser::new(),
        |record| {
            assert_eq!(record.item_identifier(), "sequence-1");
            assert_eq!(record.payload().quantity, 1);
            Ok::<(), &str>(())
        },
    );
    assert_eq!(kinesis_report.stream_checkpoint(), Some("sequence-2"));

    println!("reported {} FIFO failures", failures.len());

    Ok(())
}
