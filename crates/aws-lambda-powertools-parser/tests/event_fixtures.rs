//! Parser event fixture integration tests.

#![cfg(feature = "aws-lambda-events")]

use std::path::PathBuf;

use aws_lambda_events::event::{
    alb::AlbTargetGroupRequest,
    apigw::{ApiGatewayProxyRequest, ApiGatewayV2httpRequest, ApiGatewayWebsocketProxyRequest},
    cloudformation::CloudFormationCustomResourceRequest,
    cloudwatch_logs::LogsEvent,
    dynamodb::Event as DynamoDbEvent,
    eventbridge::EventBridgeEvent,
    firehose::KinesisFirehoseEvent,
    kinesis::KinesisEvent,
    lambda_function_urls::LambdaFunctionUrlRequest,
    s3::{S3Event, batch_job::S3BatchJobEvent, object_lambda::S3ObjectLambdaEvent},
    ses::SimpleEmailEvent,
    sns::SnsEvent,
    sqs::SqsEvent,
    vpc_lattice::{VpcLatticeRequestV1, VpcLatticeRequestV2},
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

#[derive(Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "PascalCase")]
struct CustomResourceProperties {
    bucket_name: String,
    retention_days: u32,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
struct S3BatchTask {
    task_id: Option<String>,
    s3_key: Option<String>,
    s3_version_id: Option<String>,
    s3_bucket_arn: Option<String>,
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
fn parses_api_gateway_v1_body_fixture() {
    let event = load_json_fixture::<ApiGatewayProxyRequest>(fixture("apigw-v1-order.json"))
        .expect("API Gateway v1 fixture should decode");

    let parsed = EventParser::new()
        .parse_apigw_v1_body::<OrderEvent>(event)
        .expect("fixture API Gateway v1 body should parse");

    assert_eq!(parsed.payload().order_id, "order-apigw-v1-1");
    assert_eq!(parsed.payload().quantity, 14);
}

#[test]
fn parses_api_gateway_websocket_body_fixture() {
    let event =
        load_json_fixture::<ApiGatewayWebsocketProxyRequest>(fixture("apigw-websocket-order.json"))
            .expect("API Gateway WebSocket fixture should decode");

    let parsed = EventParser::new()
        .parse_apigw_websocket_body::<OrderEvent>(event)
        .expect("fixture API Gateway WebSocket body should parse");

    assert_eq!(parsed.payload().order_id, "order-apigw-websocket-1");
    assert_eq!(parsed.payload().quantity, 15);
}

#[test]
fn parses_vpc_lattice_body_fixture() {
    let event = load_json_fixture::<VpcLatticeRequestV1>(fixture("vpc-lattice-v1-order.json"))
        .expect("VPC Lattice v1 fixture should decode");

    let parsed = EventParser::new()
        .parse_vpc_lattice_body::<OrderEvent>(event)
        .expect("fixture VPC Lattice v1 body should parse");

    assert_eq!(parsed.payload().order_id, "order-vpc-lattice-v1-1");
    assert_eq!(parsed.payload().quantity, 16);
}

#[test]
fn parses_vpc_lattice_v2_body_fixture() {
    let event = load_json_fixture::<VpcLatticeRequestV2>(fixture("vpc-lattice-v2-order.json"))
        .expect("VPC Lattice v2 fixture should decode");

    let parsed = EventParser::new()
        .parse_vpc_lattice_v2_body::<OrderEvent>(event)
        .expect("fixture VPC Lattice v2 body should parse");

    assert_eq!(parsed.payload().order_id, "order-vpc-lattice-v2-1");
    assert_eq!(parsed.payload().quantity, 17);
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

#[test]
fn parses_alb_body_fixture() {
    let event = load_json_fixture::<AlbTargetGroupRequest>(fixture("alb-order.json"))
        .expect("ALB fixture should decode");

    let parsed = EventParser::new()
        .parse_alb_body::<OrderEvent>(event)
        .expect("fixture ALB body should parse");

    assert_eq!(parsed.payload().order_id, "order-alb-1");
    assert_eq!(parsed.payload().quantity, 9);
}

#[test]
fn parses_lambda_function_url_body_fixture() {
    let event = load_json_fixture::<LambdaFunctionUrlRequest>(fixture("lambda-url-order.json"))
        .expect("Lambda Function URL fixture should decode");

    let parsed = EventParser::new()
        .parse_lambda_function_url_body::<OrderEvent>(event)
        .expect("fixture Lambda Function URL body should parse");

    assert_eq!(parsed.payload().order_id, "order-lambda-url-1");
    assert_eq!(parsed.payload().quantity, 10);
}

#[test]
fn parses_sns_message_fixture() {
    let event = load_json_fixture::<SnsEvent>(fixture("sns-orders.json"))
        .expect("SNS fixture should decode");

    let parsed = EventParser::new()
        .parse_sns_messages::<OrderEvent>(event)
        .expect("fixture SNS messages should parse");

    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].payload().order_id, "order-sns-1");
    assert_eq!(parsed[0].payload().quantity, 11);
}

#[test]
fn parses_s3_record_fixture() {
    let event = load_json_fixture::<S3Event>(fixture("s3-order-object.json"))
        .expect("S3 fixture should decode");

    let parsed = EventParser::new()
        .parse_s3_records::<Value>(event)
        .expect("fixture S3 records should parse");

    assert_eq!(parsed.len(), 1);
    assert_eq!(
        parsed[0]
            .payload()
            .pointer("/s3/bucket/name")
            .and_then(Value::as_str),
        Some("orders")
    );
    assert_eq!(
        parsed[0]
            .payload()
            .pointer("/s3/object/key")
            .and_then(Value::as_str),
        Some("orders/order-s3-1.json")
    );
}

#[test]
fn parses_s3_object_lambda_payload_fixture() {
    let event =
        load_json_fixture::<S3ObjectLambdaEvent<Value>>(fixture("s3-object-lambda-order.json"))
            .expect("S3 Object Lambda fixture should decode");

    let parsed = EventParser::new()
        .parse_s3_object_lambda_configuration_payload::<OrderEvent>(event)
        .expect("fixture S3 Object Lambda payload should parse");

    assert_eq!(
        parsed.payload(),
        &OrderEvent {
            order_id: "order-s3-object-lambda-1".to_owned(),
            quantity: 13,
        }
    );
}

#[test]
fn parses_s3_batch_job_task_fixture() {
    let event = load_json_fixture::<S3BatchJobEvent>(fixture("s3-batch-orders.json"))
        .expect("S3 Batch fixture should decode");

    let parsed = EventParser::new()
        .parse_s3_batch_job_tasks::<S3BatchTask>(event)
        .expect("fixture S3 Batch tasks should parse");

    assert_eq!(parsed.len(), 1);
    assert_eq!(
        parsed[0].payload(),
        &S3BatchTask {
            task_id: Some("task-s3-batch-1".to_owned()),
            s3_key: Some("orders/order-s3-batch-1.json".to_owned()),
            s3_version_id: Some("version-s3-batch-1".to_owned()),
            s3_bucket_arn: Some("arn:aws:s3:::orders".to_owned()),
        }
    );
}

#[test]
fn parses_s3_over_sqs_record_fixture() {
    let event = load_json_fixture::<SqsEvent>(fixture("s3-over-sqs-order-object.json"))
        .expect("S3-over-SQS fixture should decode");

    let parsed = EventParser::new()
        .parse_s3_sqs_event_records::<Value>(event)
        .expect("fixture S3-over-SQS records should parse");

    assert_eq!(parsed.len(), 1);
    assert_eq!(
        parsed[0]
            .payload()
            .pointer("/s3/object/key")
            .and_then(Value::as_str),
        Some("orders/order-s3-sqs-1.json")
    );
}

#[test]
fn parses_sns_over_sqs_message_fixture() {
    let event = load_json_fixture::<SqsEvent>(fixture("sns-over-sqs-orders.json"))
        .expect("SNS-over-SQS fixture should decode");

    let parsed = EventParser::new()
        .parse_sns_sqs_messages::<OrderEvent>(event)
        .expect("fixture SNS-over-SQS messages should parse");

    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].payload().order_id, "order-sns-sqs-1");
    assert_eq!(parsed[0].payload().quantity, 12);
}

#[test]
fn parses_ses_record_fixture() {
    let event = load_json_fixture::<SimpleEmailEvent>(fixture("ses-order-email.json"))
        .expect("SES fixture should decode");

    let parsed = EventParser::new()
        .parse_ses_records::<Value>(event)
        .expect("fixture SES records should parse");

    assert_eq!(parsed.len(), 1);
    assert_eq!(
        parsed[0]
            .payload()
            .pointer("/ses/mail/messageId")
            .and_then(Value::as_str),
        Some("message-ses-1")
    );
    assert_eq!(
        parsed[0]
            .payload()
            .pointer("/ses/mail/commonHeaders/subject")
            .and_then(Value::as_str),
        Some("Order received")
    );
}

#[test]
fn parses_cloudformation_resource_properties_fixture() {
    let event = load_json_fixture::<CloudFormationCustomResourceRequest<Value, Value>>(fixture(
        "cloudformation-bucket-policy-update.json",
    ))
    .expect("CloudFormation fixture should decode");

    let parsed = EventParser::new()
        .parse_cloudformation_resource_properties::<CustomResourceProperties>(event)
        .expect("fixture CloudFormation resource properties should parse");

    assert_eq!(parsed.payload().bucket_name, "orders");
    assert_eq!(parsed.payload().retention_days, 30);
}

#[test]
fn parses_cloudformation_old_resource_properties_fixture() {
    let event = load_json_fixture::<CloudFormationCustomResourceRequest<Value, Value>>(fixture(
        "cloudformation-bucket-policy-update.json",
    ))
    .expect("CloudFormation fixture should decode");

    let parsed = EventParser::new()
        .parse_cloudformation_old_resource_properties::<CustomResourceProperties>(event)
        .expect("fixture CloudFormation old resource properties should parse");

    assert_eq!(parsed.payload().bucket_name, "orders");
    assert_eq!(parsed.payload().retention_days, 7);
}

#[test]
fn parses_kinesis_record_fixture() {
    let event = load_json_fixture::<KinesisEvent>(fixture("kinesis-orders.json"))
        .expect("Kinesis fixture should decode");

    let parsed = EventParser::new()
        .parse_kinesis_records::<OrderEvent>(event)
        .expect("fixture Kinesis data should parse");

    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].payload().order_id, "order-kinesis-1");
    assert_eq!(parsed[0].payload().quantity, 5);
}

#[test]
fn parses_firehose_record_fixture() {
    let event = load_json_fixture::<KinesisFirehoseEvent>(fixture("firehose-orders.json"))
        .expect("Firehose fixture should decode");

    let parsed = EventParser::new()
        .parse_firehose_records::<OrderEvent>(event)
        .expect("fixture Firehose data should parse");

    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].payload().order_id, "order-firehose-1");
    assert_eq!(parsed[0].payload().quantity, 6);
}

#[test]
fn parses_cloudwatch_log_message_fixture() {
    let event = load_json_fixture::<LogsEvent>(fixture("cloudwatch-logs-orders.json"))
        .expect("CloudWatch Logs fixture should decode");

    let parsed = EventParser::new()
        .parse_cloudwatch_log_messages::<OrderEvent>(event)
        .expect("fixture log message should parse");

    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].payload().order_id, "order-log-1");
    assert_eq!(parsed[0].payload().quantity, 7);
}

#[test]
fn parses_dynamodb_new_image_fixture() {
    let event = load_json_fixture::<DynamoDbEvent>(fixture("dynamodb-orders.json"))
        .expect("DynamoDB fixture should decode");

    let parsed = EventParser::new()
        .parse_dynamodb_new_images::<OrderEvent>(event)
        .expect("fixture DynamoDB NewImage should parse");

    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].payload().order_id, "order-dynamodb-1");
    assert_eq!(parsed[0].payload().quantity, 8);
}

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("events")
        .join(name)
}
