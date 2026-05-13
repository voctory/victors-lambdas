//! Event envelope adapters.

use aws_lambda_events::event::{eventbridge::EventBridgeEvent, sns::SnsEvent, sqs::SqsEvent};
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::{EventParser, ParseError, ParseErrorKind, ParsedEvent};

impl EventParser {
    /// Parses an `EventBridge` `detail` payload.
    ///
    /// # Errors
    ///
    /// Returns a parse error when `detail` cannot be decoded into `T`.
    pub fn parse_eventbridge_detail<T>(
        &self,
        event: EventBridgeEvent<Value>,
    ) -> Result<ParsedEvent<T>, ParseError>
    where
        T: DeserializeOwned,
    {
        self.parse_json_value(event.detail)
    }

    /// Parses JSON SQS message bodies.
    ///
    /// Each record body is decoded into `T` and returned in record order.
    ///
    /// # Errors
    ///
    /// Returns a parse error when a record is missing a body or any body cannot
    /// be decoded into `T`.
    pub fn parse_sqs_message_bodies<T>(
        &self,
        event: SqsEvent,
    ) -> Result<Vec<ParsedEvent<T>>, ParseError>
    where
        T: DeserializeOwned,
    {
        event
            .records
            .into_iter()
            .enumerate()
            .map(|(index, record)| {
                let body = record.body.ok_or_else(|| {
                    ParseError::new(
                        ParseErrorKind::Data,
                        format!("SQS record at index {index} is missing body"),
                    )
                })?;
                self.parse_json_str(&body)
            })
            .collect()
    }

    /// Parses JSON SNS messages.
    ///
    /// Each record message is decoded into `T` and returned in record order.
    ///
    /// # Errors
    ///
    /// Returns a parse error when any SNS message cannot be decoded into `T`.
    pub fn parse_sns_messages<T>(&self, event: SnsEvent) -> Result<Vec<ParsedEvent<T>>, ParseError>
    where
        T: DeserializeOwned,
    {
        event
            .records
            .into_iter()
            .map(|record| self.parse_json_str(&record.sns.message))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use aws_lambda_events::event::{
        eventbridge::EventBridgeEvent,
        sns::{SnsEvent, SnsMessage, SnsRecord},
        sqs::{SqsEvent, SqsMessage},
    };
    use serde::Deserialize;
    use serde_json::{Value, json};

    use crate::{EventParser, ParseErrorKind};

    #[derive(Debug, Deserialize, Eq, PartialEq)]
    struct OrderEvent {
        order_id: String,
        quantity: u32,
    }

    #[test]
    fn parses_eventbridge_detail() {
        let mut event = EventBridgeEvent::<Value>::default();
        event.detail_type = "OrderCreated".to_owned();
        event.source = "orders".to_owned();
        event.detail = json!({
            "order_id": "order-1",
            "quantity": 2,
        });

        let parsed = EventParser::new()
            .parse_eventbridge_detail::<OrderEvent>(event)
            .expect("valid detail should parse");

        assert_eq!(
            parsed.into_payload(),
            OrderEvent {
                order_id: "order-1".to_owned(),
                quantity: 2,
            }
        );
    }

    #[test]
    fn parses_sqs_message_bodies() {
        let mut first = SqsMessage::default();
        first.body = Some(r#"{"order_id":"order-1","quantity":2}"#.to_owned());
        let mut second = SqsMessage::default();
        second.body = Some(r#"{"order_id":"order-2","quantity":3}"#.to_owned());
        let mut event = SqsEvent::default();
        event.records = vec![first, second];

        let parsed = EventParser::new()
            .parse_sqs_message_bodies::<OrderEvent>(event)
            .expect("valid bodies should parse");

        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].payload().order_id, "order-1");
        assert_eq!(parsed[1].payload().quantity, 3);
    }

    #[test]
    fn rejects_sqs_records_without_bodies() {
        let mut event = SqsEvent::default();
        event.records = vec![SqsMessage::default()];

        let error = EventParser::new()
            .parse_sqs_message_bodies::<OrderEvent>(event)
            .expect_err("missing body should fail");

        assert_eq!(error.kind(), ParseErrorKind::Data);
        assert_eq!(error.message(), "SQS record at index 0 is missing body");
    }

    #[test]
    fn parses_sns_messages() {
        let mut message = SnsMessage::default();
        message.message = r#"{"order_id":"order-1","quantity":2}"#.to_owned();
        let mut record = SnsRecord::default();
        record.sns = message;
        let mut event = SnsEvent::default();
        event.records = vec![record];

        let parsed = EventParser::new()
            .parse_sns_messages::<OrderEvent>(event)
            .expect("valid messages should parse");

        assert_eq!(parsed[0].payload().order_id, "order-1");
    }
}
