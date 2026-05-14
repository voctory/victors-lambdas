//! Parser snippet for documentation.

use std::error::Error;

use aws_lambda_events::event::sqs::{SqsEvent, SqsMessage};
use serde::Deserialize;
use victors_lambdas::prelude::{BedrockAgentEventModel, EventParser};

#[derive(Debug, Deserialize, Eq, PartialEq)]
struct OrderEvent {
    order_id: String,
    quantity: u32,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
struct VacationRequest {
    username: String,
    days: u8,
}

fn sqs_message(body: &str) -> SqsMessage {
    let mut message = SqsMessage::default();
    message.body = Some(body.to_owned());
    message
}

fn main() -> Result<(), Box<dyn Error>> {
    let parser = EventParser::new();

    let order = parser
        .parse_json_str::<OrderEvent>(r#"{"order_id":"order-1","quantity":2}"#)?
        .into_payload();
    assert_eq!(
        order,
        OrderEvent {
            order_id: "order-1".to_owned(),
            quantity: 2,
        }
    );

    let mut sqs_event = SqsEvent::default();
    sqs_event.records = vec![
        sqs_message(r#"{"order_id":"order-2","quantity":1}"#),
        sqs_message(r#"{"order_id":"order-3","quantity":4}"#),
    ];

    let orders = parser.parse_sqs_message_bodies::<OrderEvent>(sqs_event)?;
    assert_eq!(orders.len(), 2);
    assert_eq!(orders[0].payload().order_id, "order-2");
    assert_eq!(orders[1].payload().quantity, 4);

    let bedrock_event: BedrockAgentEventModel = serde_json::from_str(
        r#"{
            "messageVersion": "1.0",
            "agent": {
                "name": "TimeOffAgent",
                "id": "agent-id",
                "alias": "prod",
                "version": "1"
            },
            "inputText": "{\"username\":\"Jane\",\"days\":3}",
            "sessionId": "session-id",
            "actionGroup": "TimeOff",
            "apiPath": "/time-off",
            "httpMethod": "POST"
        }"#,
    )?;

    let request = parser
        .parse_bedrock_agent_openapi_input::<VacationRequest>(&bedrock_event)?
        .into_payload();
    assert_eq!(
        request,
        VacationRequest {
            username: "Jane".to_owned(),
            days: 3,
        }
    );

    println!(
        "parsed {} orders and {} vacation days",
        orders.len() + 1,
        request.days
    );

    Ok(())
}
