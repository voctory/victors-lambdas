//! Parser event fixture integration tests.

#![cfg(feature = "aws-lambda-events")]

use std::path::PathBuf;

use aws_lambda_events::event::{
    bedrock_agent_runtime::AgentEvent, cognito::CognitoEventUserPoolsPreSignup,
};
use aws_lambda_powertools_parser::{
    ActiveMqModel, AlbModel, ApiGatewayAuthorizerHttpApiV1Request,
    ApiGatewayAuthorizerIamPolicyResponse, ApiGatewayAuthorizerRequest,
    ApiGatewayAuthorizerRequestV2, ApiGatewayAuthorizerResponse,
    ApiGatewayAuthorizerSimpleResponse, ApiGatewayAuthorizerToken, ApiGatewayProxyEventModel,
    ApiGatewayProxyEventV2Model, ApiGatewayWebsocketConnectEvent,
    ApiGatewayWebsocketDisconnectEvent, ApiGatewayWebsocketMessageEvent, AppSyncBatchResolverEvent,
    AppSyncEventsEvent, AppSyncResolverEvent, CloudFormationCustomResourceCreate,
    CloudFormationCustomResourceDelete, CloudFormationCustomResourceRequest,
    CloudFormationCustomResourceResponse, CloudFormationCustomResourceResponseStatus,
    CloudFormationCustomResourceUpdate, CloudWatchLogsModel, DynamoDbStreamModel, EventBridgeModel,
    EventParser, KafkaMskEventModel, KafkaSelfManagedEventModel, KinesisDataStreamModel,
    KinesisFirehoseModel, KinesisFirehoseSqsModel, LambdaFunctionUrlModel, RabbitMqModel,
    S3BatchOperationModel, S3Model, S3ObjectLambdaEvent, S3SqsEventNotificationModel, SesModel,
    SnsModel, SqsModel, VpcLatticeModel, VpcLatticeV2Model,
};
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
struct CognitoUserAttributes {
    sub: String,
    email: String,
    #[serde(rename = "custom:tenant")]
    custom_tenant: String,
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
    let event = load_json_fixture::<ApiGatewayProxyEventV2Model>(fixture("apigw-v2-order.json"))
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
    let event = load_json_fixture::<ApiGatewayProxyEventModel>(fixture("apigw-v1-order.json"))
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
        load_json_fixture::<ApiGatewayWebsocketMessageEvent>(fixture("apigw-websocket-order.json"))
            .expect("API Gateway WebSocket fixture should decode");

    let parsed = EventParser::new()
        .parse_apigw_websocket_body::<OrderEvent>(event)
        .expect("fixture API Gateway WebSocket body should parse");

    assert_eq!(parsed.payload().order_id, "order-apigw-websocket-1");
    assert_eq!(parsed.payload().quantity, 15);
}

#[test]
fn parses_api_gateway_websocket_lifecycle_fixtures() {
    let connect = load_json_fixture::<ApiGatewayWebsocketConnectEvent>(fixture(
        "apigw-websocket-connect.json",
    ))
    .expect("API Gateway WebSocket connect fixture should decode");
    let disconnect = load_json_fixture::<ApiGatewayWebsocketDisconnectEvent>(fixture(
        "apigw-websocket-disconnect.json",
    ))
    .expect("API Gateway WebSocket disconnect fixture should decode");

    assert_eq!(
        connect.request_context.event_type.as_deref(),
        Some("CONNECT")
    );
    assert_eq!(
        connect.request_context.route_key.as_deref(),
        Some("$connect")
    );
    assert_eq!(
        disconnect.request_context.event_type.as_deref(),
        Some("DISCONNECT")
    );
    assert_eq!(
        disconnect.request_context.route_key.as_deref(),
        Some("$disconnect")
    );
}

#[test]
fn parses_api_gateway_token_authorizer_fixture() {
    let event =
        load_json_fixture::<ApiGatewayAuthorizerToken>(fixture("apigw-authorizer-token.json"))
            .expect("API Gateway TOKEN authorizer fixture should decode");

    assert_eq!(event.type_.as_deref(), Some("TOKEN"));
    assert_eq!(event.authorization_token.as_deref(), Some("allow"));
    assert_eq!(
        event.method_arn.as_deref(),
        Some("arn:aws:execute-api:us-west-2:123456789012:api-id/prod/GET/orders")
    );
}

#[test]
fn parses_api_gateway_request_authorizer_fixture() {
    let event =
        load_json_fixture::<ApiGatewayAuthorizerRequest>(fixture("apigw-authorizer-request.json"))
            .expect("API Gateway REQUEST authorizer fixture should decode");

    assert_eq!(event.type_.as_deref(), Some("REQUEST"));
    assert_eq!(event.path.as_deref(), Some("/orders/123"));
    assert_eq!(
        event.request_context.account_id.as_deref(),
        Some("123456789012")
    );
}

#[test]
fn parses_api_gateway_http_api_v1_authorizer_fixture() {
    let event = load_json_fixture::<ApiGatewayAuthorizerHttpApiV1Request>(fixture(
        "apigw-authorizer-http-api-v1-request.json",
    ))
    .expect("API Gateway HTTP API v1 authorizer fixture should decode");

    assert_eq!(event.version.as_deref(), Some("1.0"));
    assert_eq!(event.identity_source.as_deref(), Some("Bearer allow"));
    assert_eq!(event.request_context.http_method.as_str(), "GET");
}

#[test]
fn parses_api_gateway_v2_authorizer_fixture() {
    let event = load_json_fixture::<ApiGatewayAuthorizerRequestV2>(fixture(
        "apigw-authorizer-v2-request.json",
    ))
    .expect("API Gateway v2 authorizer fixture should decode");

    assert_eq!(event.version.as_deref(), Some("2.0"));
    assert_eq!(
        event.route_arn.as_deref(),
        Some("arn:aws:execute-api:us-west-2:123456789012:api-id/prod/GET/orders/123")
    );
    assert_eq!(event.request_context.http.method.as_str(), "GET");
}

#[test]
fn parses_api_gateway_authorizer_response_fixtures() {
    let response = load_json_fixture::<ApiGatewayAuthorizerResponse>(fixture(
        "apigw-authorizer-iam-response.json",
    ))
    .expect("API Gateway authorizer IAM response fixture should decode");
    let http_api_response = load_json_fixture::<ApiGatewayAuthorizerIamPolicyResponse>(fixture(
        "apigw-authorizer-iam-response.json",
    ))
    .expect("API Gateway HTTP API IAM response fixture should decode");
    let simple_response = load_json_fixture::<ApiGatewayAuthorizerSimpleResponse>(fixture(
        "apigw-authorizer-simple-response.json",
    ))
    .expect("API Gateway HTTP API simple response fixture should decode");

    assert_eq!(response.principal_id.as_deref(), Some("user-123"));
    assert_eq!(http_api_response.principal_id.as_deref(), Some("user-123"));
    assert!(simple_response.is_authorized);
}

#[test]
fn parses_vpc_lattice_body_fixture() {
    let event = load_json_fixture::<VpcLatticeModel>(fixture("vpc-lattice-v1-order.json"))
        .expect("VPC Lattice v1 fixture should decode");

    let parsed = EventParser::new()
        .parse_vpc_lattice_body::<OrderEvent>(event)
        .expect("fixture VPC Lattice v1 body should parse");

    assert_eq!(parsed.payload().order_id, "order-vpc-lattice-v1-1");
    assert_eq!(parsed.payload().quantity, 16);
}

#[test]
fn parses_vpc_lattice_v2_body_fixture() {
    let event = load_json_fixture::<VpcLatticeV2Model>(fixture("vpc-lattice-v2-order.json"))
        .expect("VPC Lattice v2 fixture should decode");

    let parsed = EventParser::new()
        .parse_vpc_lattice_v2_body::<OrderEvent>(event)
        .expect("fixture VPC Lattice v2 body should parse");

    assert_eq!(parsed.payload().order_id, "order-vpc-lattice-v2-1");
    assert_eq!(parsed.payload().quantity, 17);
}

#[test]
fn parses_eventbridge_detail_fixture() {
    let event = load_json_fixture::<EventBridgeModel<Value>>(fixture("eventbridge-order.json"))
        .expect("EventBridge fixture should decode");

    let parsed = EventParser::new()
        .parse_eventbridge_detail::<OrderEvent>(event)
        .expect("fixture detail should parse");

    assert_eq!(parsed.payload().order_id, "order-eventbridge-1");
    assert_eq!(parsed.payload().quantity, 3);
}

#[test]
fn parses_eventbridge_scheduler_empty_detail_fixture() {
    let event = load_json_fixture::<EventBridgeModel<Value>>(fixture(
        "eventbridge-scheduler-empty-detail.json",
    ))
    .expect("EventBridge Scheduler fixture should decode");

    let parsed = EventParser::new()
        .parse_eventbridge_detail::<Value>(event)
        .expect("fixture Scheduler detail should parse");

    assert_eq!(parsed.into_payload(), serde_json::json!({}));
}

#[test]
fn parses_appsync_arguments_fixture() {
    let event = load_json_fixture::<AppSyncResolverEvent<Value, Value, Value>>(fixture(
        "appsync-direct-order.json",
    ))
    .expect("AppSync direct resolver fixture should decode");

    let parsed = EventParser::new()
        .parse_appsync_arguments::<OrderEvent>(event)
        .expect("fixture AppSync arguments should parse");

    assert_eq!(parsed.payload().order_id, "order-appsync-args-1");
    assert_eq!(parsed.payload().quantity, 21);
}

#[test]
fn parses_appsync_source_fixture() {
    let event = load_json_fixture::<AppSyncResolverEvent<Value, Value, Value>>(fixture(
        "appsync-direct-order.json",
    ))
    .expect("AppSync direct resolver fixture should decode");

    let parsed = EventParser::new()
        .parse_appsync_source::<OrderEvent>(event)
        .expect("fixture AppSync source should parse");

    assert_eq!(parsed.payload().order_id, "order-appsync-source-1");
    assert_eq!(parsed.payload().quantity, 22);
}

#[test]
fn parses_appsync_batch_resolver_fixture() {
    let event = load_json_fixture::<AppSyncBatchResolverEvent<Value, Value, Value>>(fixture(
        "appsync-direct-batch-orders.json",
    ))
    .expect("AppSync batch resolver fixture should decode");

    assert_eq!(event.len(), 2);
    assert_eq!(
        event[0]
            .arguments
            .as_ref()
            .and_then(|arguments| arguments.pointer("/order_id"))
            .and_then(Value::as_str),
        Some("order-appsync-batch-1")
    );
    assert_eq!(
        event[1]
            .source
            .as_ref()
            .and_then(|source| source.pointer("/quantity"))
            .and_then(Value::as_u64),
        Some(28)
    );
}

#[test]
fn parses_appsync_events_payload_fixture() {
    let event = load_json_fixture::<AppSyncEventsEvent>(fixture("appsync-events-orders.json"))
        .expect("AppSync Events fixture should decode");

    let parsed = EventParser::new()
        .parse_appsync_events_payloads::<OrderEvent>(event)
        .expect("fixture AppSync Events payloads should parse");

    assert_eq!(parsed.len(), 2);
    assert_eq!(parsed[0].payload().order_id, "order-appsync-events-1");
    assert_eq!(parsed[0].payload().quantity, 23);
    assert_eq!(parsed[1].payload().order_id, "order-appsync-events-2");
    assert_eq!(parsed[1].payload().quantity, 24);
}

#[test]
fn parses_bedrock_agent_input_fixture() {
    let event = load_json_fixture::<AgentEvent>(fixture("bedrock-agent-order.json"))
        .expect("Bedrock Agent fixture should decode");

    let parsed = EventParser::new()
        .parse_bedrock_agent_input::<OrderEvent>(event)
        .expect("fixture Bedrock Agent input should parse");

    assert_eq!(parsed.payload().order_id, "order-bedrock-agent-1");
    assert_eq!(parsed.payload().quantity, 25);
}

#[test]
fn parses_cognito_pre_signup_user_attributes_fixture() {
    let event = load_json_fixture::<CognitoEventUserPoolsPreSignup>(fixture(
        "cognito-pre-signup-user.json",
    ))
    .expect("Cognito Pre sign-up fixture should decode");

    let parsed = EventParser::new()
        .parse_cognito_pre_signup_user_attributes::<CognitoUserAttributes>(event)
        .expect("fixture Cognito user attributes should parse");

    assert_eq!(parsed.payload().sub, "user-cognito-1");
    assert_eq!(parsed.payload().email, "user@example.com");
    assert_eq!(parsed.payload().custom_tenant, "orders");
}

#[test]
fn parses_sqs_message_body_fixture() {
    let event = load_json_fixture::<SqsModel>(fixture("sqs-orders.json"))
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
fn parses_activemq_message_data_fixture() {
    let event = load_json_fixture::<ActiveMqModel>(fixture("activemq-orders.json"))
        .expect("ActiveMQ fixture should decode");

    let parsed = EventParser::new()
        .parse_activemq_message_data::<OrderEvent>(event)
        .expect("fixture ActiveMQ data should parse");

    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].payload().order_id, "order-active-mq-1");
    assert_eq!(parsed[0].payload().quantity, 26);
}

#[test]
fn parses_rabbitmq_message_data_fixture() {
    let event = load_json_fixture::<RabbitMqModel>(fixture("rabbitmq-orders.json"))
        .expect("RabbitMQ fixture should decode");

    let parsed = EventParser::new()
        .parse_rabbitmq_message_data::<OrderEvent>(event)
        .expect("fixture RabbitMQ data should parse");

    assert_eq!(parsed["orders::/"].len(), 1);
    assert_eq!(
        parsed["orders::/"][0].payload().order_id,
        "order-rabbit-mq-1"
    );
    assert_eq!(parsed["orders::/"][0].payload().quantity, 27);
}

#[test]
fn parses_alb_body_fixture() {
    let event = load_json_fixture::<AlbModel>(fixture("alb-order.json"))
        .expect("ALB fixture should decode");

    let parsed = EventParser::new()
        .parse_alb_body::<OrderEvent>(event)
        .expect("fixture ALB body should parse");

    assert_eq!(parsed.payload().order_id, "order-alb-1");
    assert_eq!(parsed.payload().quantity, 9);
}

#[test]
fn parses_lambda_function_url_body_fixture() {
    let event = load_json_fixture::<LambdaFunctionUrlModel>(fixture("lambda-url-order.json"))
        .expect("Lambda Function URL fixture should decode");

    let parsed = EventParser::new()
        .parse_lambda_function_url_body::<OrderEvent>(event)
        .expect("fixture Lambda Function URL body should parse");

    assert_eq!(parsed.payload().order_id, "order-lambda-url-1");
    assert_eq!(parsed.payload().quantity, 10);
}

#[test]
fn parses_sns_message_fixture() {
    let event = load_json_fixture::<SnsModel>(fixture("sns-orders.json"))
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
    let event = load_json_fixture::<S3Model>(fixture("s3-order-object.json"))
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
    let event = load_json_fixture::<S3BatchOperationModel>(fixture("s3-batch-orders.json"))
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
    let event =
        load_json_fixture::<S3SqsEventNotificationModel>(fixture("s3-over-sqs-order-object.json"))
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
    let event = load_json_fixture::<SqsModel>(fixture("sns-over-sqs-orders.json"))
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
    let event = load_json_fixture::<SesModel>(fixture("ses-order-email.json"))
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
fn parses_cloudformation_request_type_fixtures() {
    let create = load_json_fixture::<CloudFormationCustomResourceCreate>(fixture(
        "cloudformation-bucket-policy-create.json",
    ))
    .expect("CloudFormation create fixture should decode");
    let update = load_json_fixture::<CloudFormationCustomResourceUpdate>(fixture(
        "cloudformation-bucket-policy-update.json",
    ))
    .expect("CloudFormation update fixture should decode");
    let delete = load_json_fixture::<CloudFormationCustomResourceDelete>(fixture(
        "cloudformation-bucket-policy-delete.json",
    ))
    .expect("CloudFormation delete fixture should decode");

    assert_eq!(create.request_id, "request-cloudformation-create-1");
    assert_eq!(
        create
            .resource_properties
            .pointer("/BucketName")
            .and_then(Value::as_str),
        Some("orders")
    );
    assert_eq!(
        update
            .old_resource_properties
            .pointer("/RetentionDays")
            .and_then(Value::as_u64),
        Some(7)
    );
    assert_eq!(delete.physical_resource_id, "bucket-policy-1");
}

#[test]
fn parses_cloudformation_response_fixture() {
    let response = load_json_fixture::<CloudFormationCustomResourceResponse>(fixture(
        "cloudformation-response-success.json",
    ))
    .expect("CloudFormation response fixture should decode");

    assert_eq!(
        response.status,
        CloudFormationCustomResourceResponseStatus::Success
    );
    assert_eq!(response.physical_resource_id, "bucket-policy-1");
    assert_eq!(
        response.data.get("BucketName").map(String::as_str),
        Some("orders")
    );
}

#[test]
fn parses_kinesis_record_fixture() {
    let event = load_json_fixture::<KinesisDataStreamModel>(fixture("kinesis-orders.json"))
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
    let event = load_json_fixture::<KinesisFirehoseModel>(fixture("firehose-orders.json"))
        .expect("Firehose fixture should decode");

    let parsed = EventParser::new()
        .parse_firehose_records::<OrderEvent>(event)
        .expect("fixture Firehose data should parse");

    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].payload().order_id, "order-firehose-1");
    assert_eq!(parsed[0].payload().quantity, 6);
}

#[test]
fn parses_firehose_sqs_message_body_fixture() {
    let event = load_json_fixture::<KinesisFirehoseSqsModel>(fixture("firehose-sqs-orders.json"))
        .expect("Firehose-delivered SQS fixture should decode");

    let parsed = EventParser::new()
        .parse_firehose_sqs_message_bodies::<OrderEvent>(event)
        .expect("fixture Firehose-delivered SQS body should parse");

    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].payload().order_id, "order-firehose-sqs-1");
    assert_eq!(parsed[0].payload().quantity, 18);
}

#[test]
fn parses_cloudwatch_log_message_fixture() {
    let event = load_json_fixture::<CloudWatchLogsModel>(fixture("cloudwatch-logs-orders.json"))
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
    let event = load_json_fixture::<DynamoDbStreamModel>(fixture("dynamodb-orders.json"))
        .expect("DynamoDB fixture should decode");

    let parsed = EventParser::new()
        .parse_dynamodb_new_images::<OrderEvent>(event)
        .expect("fixture DynamoDB NewImage should parse");

    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].payload().order_id, "order-dynamodb-1");
    assert_eq!(parsed[0].payload().quantity, 8);
}

#[test]
fn parses_kinesis_dynamodb_new_image_fixture() {
    let event =
        load_json_fixture::<KinesisDataStreamModel>(fixture("kinesis-dynamodb-orders.json"))
            .expect("Kinesis-delivered DynamoDB fixture should decode");

    let parsed = EventParser::new()
        .parse_kinesis_dynamodb_new_images::<OrderEvent>(event)
        .expect("fixture Kinesis-delivered DynamoDB NewImage should parse");

    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].payload().order_id, "order-kinesis-dynamodb-1");
    assert_eq!(parsed[0].payload().quantity, 19);
}

#[test]
fn parses_kafka_record_value_fixture() {
    let event = load_json_fixture::<KafkaMskEventModel>(fixture("kafka-orders.json"))
        .expect("Kafka fixture should decode");

    let parsed = EventParser::new()
        .parse_kafka_record_values::<OrderEvent>(event)
        .expect("fixture Kafka record value should parse");

    assert_eq!(parsed["orders-0"].len(), 1);
    assert_eq!(parsed["orders-0"][0].payload().order_id, "order-kafka-1");
    assert_eq!(parsed["orders-0"][0].payload().quantity, 20);
}

#[test]
fn parses_self_managed_kafka_record_value_fixture() {
    let event = load_json_fixture::<KafkaSelfManagedEventModel>(fixture("kafka-orders.json"))
        .expect("Kafka fixture should decode through self-managed alias");

    let parsed = EventParser::new()
        .parse_kafka_record_values::<OrderEvent>(event)
        .expect("fixture Kafka record value should parse");

    assert_eq!(parsed["orders-0"].len(), 1);
    assert_eq!(parsed["orders-0"][0].payload().order_id, "order-kafka-1");
    assert_eq!(parsed["orders-0"][0].payload().quantity, 20);
}

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("events")
        .join(name)
}
