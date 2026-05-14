//! Parser event fixture integration tests.

#![cfg(feature = "aws-lambda-events")]

use std::path::PathBuf;

use aws_lambda_events::event::{
    apigw::ApiGatewayV2httpRequest, eventbridge::EventBridgeEvent, sqs::SqsEvent,
};
use aws_lambda_powertools_parser::EventParser;
use aws_lambda_powertools_testing::load_json_fixture;
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize, Eq, PartialEq)]
struct OrderEvent {
    order_id: String,
    quantity: u32,
}

#[test]
fn parses_api_gateway_v2_body_fixture() {
    let event = load_json_fixture::<ApiGatewayV2httpRequest>(fixture("apigw-v2-order.json"))
        .expect("API Gateway v2 fixture should decode");

    let parsed = EventParser::new()
        .parse_apigw_v2_body::<OrderEvent>(event)
        .expect("fixture body should parse");

    assert_eq!(
        parsed.into_payload(),
        OrderEvent {
            order_id: "order-apigw-1".to_owned(),
            quantity: 2,
        }
    );
}

#[test]
fn parses_eventbridge_detail_fixture() {
    let event = load_json_fixture::<EventBridgeEvent<Value>>(fixture("eventbridge-order.json"))
        .expect("EventBridge fixture should decode");

    let parsed = EventParser::new()
        .parse_eventbridge_detail::<OrderEvent>(event)
        .expect("fixture detail should parse");

    assert_eq!(parsed.payload().order_id, "order-eventbridge-1");
    assert_eq!(parsed.payload().quantity, 3);
}

#[test]
fn parses_sqs_message_body_fixture() {
    let event = load_json_fixture::<SqsEvent>(fixture("sqs-orders.json"))
        .expect("SQS fixture should decode");

    let parsed = EventParser::new()
        .parse_sqs_message_bodies::<OrderEvent>(event)
        .expect("fixture message bodies should parse");

    assert_eq!(parsed.len(), 2);
    assert_eq!(parsed[0].payload().order_id, "order-sqs-1");
    assert_eq!(parsed[0].payload().quantity, 1);
    assert_eq!(parsed[1].payload().order_id, "order-sqs-2");
    assert_eq!(parsed[1].payload().quantity, 4);
}

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("events")
        .join(name)
}
