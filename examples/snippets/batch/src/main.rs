//! Batch snippet for documentation.

use std::error::Error;

use aws_lambda_events::event::sqs::{SqsEvent, SqsMessage};
use aws_lambda_powertools::prelude::{BatchProcessor, BatchRecord};

fn sqs_message(id: &str, body: &str) -> SqsMessage {
    let mut message = SqsMessage::default();
    message.message_id = Some(id.to_owned());
    message.body = Some(body.to_owned());
    message
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

    println!("reported {} FIFO failures", failures.len());

    Ok(())
}
