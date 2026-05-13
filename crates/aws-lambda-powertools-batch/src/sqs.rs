//! SQS batch adapters.

use std::fmt::Display;

use aws_lambda_events::event::sqs::{SqsEvent, SqsMessage};

use crate::{BatchProcessingReport, BatchProcessor, BatchRecordResult, BatchResponse};

impl BatchProcessor {
    /// Processes SQS records and builds a batch processing report.
    ///
    /// Failed records are identified by their SQS `message_id`, matching AWS
    /// Lambda partial batch response semantics.
    pub fn process_sqs<E>(
        &self,
        event: &SqsEvent,
        mut handler: impl FnMut(&SqsMessage) -> Result<(), E>,
    ) -> BatchProcessingReport
    where
        E: Display,
    {
        let results = event.records.iter().enumerate().map(|(index, record)| {
            let item_identifier = sqs_item_identifier(index, record);
            match handler(record) {
                Ok(()) => BatchRecordResult::success(item_identifier),
                Err(error) => BatchRecordResult::failure(item_identifier, error.to_string()),
            }
        });

        BatchProcessingReport::from_results(results)
    }

    /// Processes SQS records and returns an AWS Lambda partial batch response.
    pub fn process_sqs_response<E>(
        &self,
        event: &SqsEvent,
        handler: impl FnMut(&SqsMessage) -> Result<(), E>,
    ) -> BatchResponse
    where
        E: Display,
    {
        self.process_sqs(event, handler).response()
    }

    /// Processes FIFO SQS records with early-stop failure semantics.
    ///
    /// After the first handler failure, remaining records are marked as failed
    /// without calling the handler. This preserves FIFO ordering by asking AWS
    /// Lambda to return the failed record and all later records to the queue.
    pub fn process_sqs_fifo<E>(
        &self,
        event: &SqsEvent,
        mut handler: impl FnMut(&SqsMessage) -> Result<(), E>,
    ) -> BatchProcessingReport
    where
        E: Display,
    {
        let mut stopped = false;
        let results = event.records.iter().enumerate().map(|(index, record)| {
            let item_identifier = sqs_item_identifier(index, record);
            if stopped {
                return BatchRecordResult::failure(
                    item_identifier,
                    "skipped after previous FIFO record failure",
                );
            }

            match handler(record) {
                Ok(()) => BatchRecordResult::success(item_identifier),
                Err(error) => {
                    stopped = true;
                    BatchRecordResult::failure(item_identifier, error.to_string())
                }
            }
        });

        BatchProcessingReport::from_results(results)
    }

    /// Processes FIFO SQS records and returns an AWS Lambda partial batch response.
    pub fn process_sqs_fifo_response<E>(
        &self,
        event: &SqsEvent,
        handler: impl FnMut(&SqsMessage) -> Result<(), E>,
    ) -> BatchResponse
    where
        E: Display,
    {
        self.process_sqs_fifo(event, handler).response()
    }
}

fn sqs_item_identifier(index: usize, record: &SqsMessage) -> String {
    record
        .message_id
        .clone()
        .unwrap_or_else(|| format!("record-{index}"))
}

#[cfg(test)]
mod tests {
    use aws_lambda_events::event::sqs::{SqsEvent, SqsMessage};
    use serde_json::json;

    use crate::{BatchItemFailure, BatchProcessor, BatchRecordResult};

    fn message(id: &str, body: &str) -> SqsMessage {
        let mut message = SqsMessage::default();
        message.message_id = Some(id.to_owned());
        message.body = Some(body.to_owned());
        message
    }

    #[test]
    fn process_sqs_uses_message_id_failures() {
        let mut event = SqsEvent::default();
        event.records = vec![message("message-1", "ok"), message("message-2", "fail")];

        let report = BatchProcessor::new().process_sqs(&event, |record| {
            if record.body.as_deref() == Some("fail") {
                Err("handler failed")
            } else {
                Ok(())
            }
        });

        assert_eq!(
            report.results(),
            &[
                BatchRecordResult::success("message-1"),
                BatchRecordResult::failure("message-2", "handler failed"),
            ]
        );
        assert_eq!(
            serde_json::to_value(report.response()).expect("response serializes"),
            json!({
                "batchItemFailures": [
                    {
                        "itemIdentifier": "message-2",
                    },
                ],
            })
        );
    }

    #[test]
    fn process_sqs_fifo_marks_remaining_records_failed_after_first_error() {
        let mut event = SqsEvent::default();
        event.records = vec![
            message("message-1", "ok"),
            message("message-2", "fail"),
            message("message-3", "not-called"),
        ];
        let mut calls = 0;

        let report = BatchProcessor::new().process_sqs_fifo(&event, |record| {
            calls += 1;
            if record.body.as_deref() == Some("fail") {
                Err("handler failed")
            } else {
                Ok(())
            }
        });

        assert_eq!(calls, 2);
        assert_eq!(
            report.results(),
            &[
                BatchRecordResult::success("message-1"),
                BatchRecordResult::failure("message-2", "handler failed"),
                BatchRecordResult::failure(
                    "message-3",
                    "skipped after previous FIFO record failure"
                ),
            ]
        );
        assert_eq!(
            report
                .response()
                .batch_item_failures()
                .iter()
                .map(BatchItemFailure::item_identifier)
                .collect::<Vec<_>>(),
            vec!["message-2", "message-3"]
        );
    }
}
