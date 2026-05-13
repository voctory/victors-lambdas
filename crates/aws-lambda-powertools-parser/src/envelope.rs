//! Event envelope adapters.

use std::collections::HashMap;

use aws_lambda_events::{
    encodings::Body,
    event::{
        alb::AlbTargetGroupRequest,
        apigw::{ApiGatewayProxyRequest, ApiGatewayV2httpRequest, ApiGatewayWebsocketProxyRequest},
        appsync::AppSyncDirectResolverEvent,
        cloudwatch_logs::LogsEvent,
        dynamodb::Event as DynamoDbEvent,
        eventbridge::EventBridgeEvent,
        firehose::KinesisFirehoseEvent,
        kafka::{KafkaEvent, KafkaRecord},
        kinesis::KinesisEvent,
        lambda_function_urls::LambdaFunctionUrlRequest,
        sns::SnsEvent,
        sqs::SqsEvent,
        vpc_lattice::{VpcLatticeRequestV1, VpcLatticeRequestV2},
    },
};
use base64::Engine;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::{EventParser, ParseError, ParseErrorKind, ParsedEvent};

impl EventParser {
    /// Parses an API Gateway REST API v1 JSON body.
    ///
    /// # Errors
    ///
    /// Returns a parse error when the body is missing, cannot be base64
    /// decoded, or cannot be decoded into `T`.
    pub fn parse_apigw_v1_body<T>(
        &self,
        event: ApiGatewayProxyRequest,
    ) -> Result<ParsedEvent<T>, ParseError>
    where
        T: DeserializeOwned,
    {
        let body = gateway_body("API Gateway v1", event.body, event.is_base64_encoded)?;
        self.parse_json_slice(&body)
    }

    /// Parses an API Gateway HTTP API v2 JSON body.
    ///
    /// # Errors
    ///
    /// Returns a parse error when the body is missing, cannot be base64
    /// decoded, or cannot be decoded into `T`.
    pub fn parse_apigw_v2_body<T>(
        &self,
        event: ApiGatewayV2httpRequest,
    ) -> Result<ParsedEvent<T>, ParseError>
    where
        T: DeserializeOwned,
    {
        let body = gateway_body("API Gateway v2", event.body, event.is_base64_encoded)?;
        self.parse_json_slice(&body)
    }

    /// Parses an API Gateway WebSocket API JSON body.
    ///
    /// # Errors
    ///
    /// Returns a parse error when the body is missing, cannot be base64
    /// decoded, or cannot be decoded into `T`.
    pub fn parse_apigw_websocket_body<T>(
        &self,
        event: ApiGatewayWebsocketProxyRequest,
    ) -> Result<ParsedEvent<T>, ParseError>
    where
        T: DeserializeOwned,
    {
        let body = gateway_body("API Gateway WebSocket", event.body, event.is_base64_encoded)?;
        self.parse_json_slice(&body)
    }

    /// Parses `AppSync` direct resolver arguments.
    ///
    /// # Errors
    ///
    /// Returns a parse error when arguments are missing or cannot be decoded
    /// into `T`.
    pub fn parse_appsync_arguments<T>(
        &self,
        event: AppSyncDirectResolverEvent<Value, Value, Value>,
    ) -> Result<ParsedEvent<T>, ParseError>
    where
        T: DeserializeOwned,
    {
        let arguments = event.arguments.ok_or_else(|| {
            ParseError::new(ParseErrorKind::Data, "AppSync event is missing arguments")
        })?;
        self.parse_json_value(arguments)
    }

    /// Parses an `AppSync` direct resolver source object.
    ///
    /// # Errors
    ///
    /// Returns a parse error when the source is missing or cannot be decoded
    /// into `T`.
    pub fn parse_appsync_source<T>(
        &self,
        event: AppSyncDirectResolverEvent<Value, Value, Value>,
    ) -> Result<ParsedEvent<T>, ParseError>
    where
        T: DeserializeOwned,
    {
        let source = event.source.ok_or_else(|| {
            ParseError::new(ParseErrorKind::Data, "AppSync event is missing source")
        })?;
        self.parse_json_value(source)
    }

    /// Parses an Application Load Balancer target group JSON body.
    ///
    /// # Errors
    ///
    /// Returns a parse error when the body is missing, cannot be base64
    /// decoded, or cannot be decoded into `T`.
    pub fn parse_alb_body<T>(
        &self,
        event: AlbTargetGroupRequest,
    ) -> Result<ParsedEvent<T>, ParseError>
    where
        T: DeserializeOwned,
    {
        let body = gateway_body("ALB", event.body, event.is_base64_encoded)?;
        self.parse_json_slice(&body)
    }

    /// Parses a Lambda Function URL JSON body.
    ///
    /// # Errors
    ///
    /// Returns a parse error when the body is missing, cannot be base64
    /// decoded, or cannot be decoded into `T`.
    pub fn parse_lambda_function_url_body<T>(
        &self,
        event: LambdaFunctionUrlRequest,
    ) -> Result<ParsedEvent<T>, ParseError>
    where
        T: DeserializeOwned,
    {
        let body = gateway_body("Lambda Function URL", event.body, event.is_base64_encoded)?;
        self.parse_json_slice(&body)
    }

    /// Parses an Amazon VPC Lattice v1 JSON body.
    ///
    /// # Errors
    ///
    /// Returns a parse error when the body is missing, cannot be base64
    /// decoded, or cannot be decoded into `T`.
    pub fn parse_vpc_lattice_body<T>(
        &self,
        event: VpcLatticeRequestV1,
    ) -> Result<ParsedEvent<T>, ParseError>
    where
        T: DeserializeOwned,
    {
        let body = event_body("VPC Lattice", event.body, event.is_base64_encoded)?;
        self.parse_json_slice(&body)
    }

    /// Parses an Amazon VPC Lattice v2 JSON body.
    ///
    /// # Errors
    ///
    /// Returns a parse error when the body is missing, cannot be base64
    /// decoded, or cannot be decoded into `T`.
    pub fn parse_vpc_lattice_v2_body<T>(
        &self,
        event: VpcLatticeRequestV2,
    ) -> Result<ParsedEvent<T>, ParseError>
    where
        T: DeserializeOwned,
    {
        let body = gateway_body("VPC Lattice v2", event.body, event.is_base64_encoded)?;
        self.parse_json_slice(&body)
    }

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

    /// Parses JSON `CloudWatch Logs` event messages.
    ///
    /// Each decoded log event message is decoded into `T` and returned in log
    /// event order. The `aws_lambda_events` model base64-decodes and
    /// decompresses `CloudWatch Logs` data during event deserialization.
    ///
    /// # Errors
    ///
    /// Returns a parse error when any log event message cannot be decoded into
    /// `T`.
    pub fn parse_cloudwatch_log_messages<T>(
        &self,
        event: LogsEvent,
    ) -> Result<Vec<ParsedEvent<T>>, ParseError>
    where
        T: DeserializeOwned,
    {
        event
            .aws_logs
            .data
            .log_events
            .into_iter()
            .map(|entry| self.parse_json_str(&entry.message))
            .collect()
    }

    /// Parses JSON Kinesis record data.
    ///
    /// Each record data blob is decoded into `T` and returned in record order.
    /// The `aws_lambda_events` model base64-decodes Kinesis data during event
    /// deserialization, so this method parses the decoded bytes directly.
    ///
    /// # Errors
    ///
    /// Returns a parse error when any record data cannot be decoded into `T`.
    pub fn parse_kinesis_records<T>(
        &self,
        event: KinesisEvent,
    ) -> Result<Vec<ParsedEvent<T>>, ParseError>
    where
        T: DeserializeOwned,
    {
        event
            .records
            .into_iter()
            .map(|record| self.parse_json_slice(&record.kinesis.data))
            .collect()
    }

    /// Parses JSON Kinesis Firehose record data.
    ///
    /// Each record data blob is decoded into `T` and returned in record order.
    /// The `aws_lambda_events` model base64-decodes Firehose data during event
    /// deserialization, so this method parses the decoded bytes directly.
    ///
    /// # Errors
    ///
    /// Returns a parse error when any record data cannot be decoded into `T`.
    pub fn parse_firehose_records<T>(
        &self,
        event: KinesisFirehoseEvent,
    ) -> Result<Vec<ParsedEvent<T>>, ParseError>
    where
        T: DeserializeOwned,
    {
        event
            .records
            .into_iter()
            .map(|record| self.parse_json_slice(&record.data))
            .collect()
    }

    /// Parses `DynamoDB` stream `NewImage` records.
    ///
    /// Each non-empty `NewImage` item is decoded into `T` with `serde_dynamo`
    /// and returned in record order.
    ///
    /// # Errors
    ///
    /// Returns a parse error when any record is missing a `NewImage` or an
    /// image cannot be decoded into `T`.
    pub fn parse_dynamodb_new_images<T>(
        &self,
        event: DynamoDbEvent,
    ) -> Result<Vec<ParsedEvent<T>>, ParseError>
    where
        T: DeserializeOwned,
    {
        event
            .records
            .into_iter()
            .enumerate()
            .map(|(index, record)| dynamodb_image("NewImage", index, record.change.new_image))
            .collect()
    }

    /// Parses `DynamoDB` stream `OldImage` records.
    ///
    /// Each non-empty `OldImage` item is decoded into `T` with `serde_dynamo`
    /// and returned in record order.
    ///
    /// # Errors
    ///
    /// Returns a parse error when any record is missing an `OldImage` or an
    /// image cannot be decoded into `T`.
    pub fn parse_dynamodb_old_images<T>(
        &self,
        event: DynamoDbEvent,
    ) -> Result<Vec<ParsedEvent<T>>, ParseError>
    where
        T: DeserializeOwned,
    {
        event
            .records
            .into_iter()
            .enumerate()
            .map(|(index, record)| dynamodb_image("OldImage", index, record.change.old_image))
            .collect()
    }

    /// Parses JSON Kafka record values.
    ///
    /// Kafka records are returned with the same topic-partition grouping used
    /// by the Lambda event. Each record value is base64-decoded before being
    /// decoded into `T`.
    ///
    /// # Errors
    ///
    /// Returns a parse error when any record value is missing, is not valid
    /// base64, or cannot be decoded into `T`.
    pub fn parse_kafka_record_values<T>(
        &self,
        event: KafkaEvent,
    ) -> Result<HashMap<String, Vec<ParsedEvent<T>>>, ParseError>
    where
        T: DeserializeOwned,
    {
        event
            .records
            .into_iter()
            .map(|(source, records)| {
                let parsed_records = records
                    .into_iter()
                    .enumerate()
                    .map(|(index, record)| {
                        let value = kafka_record_value(&source, index, record)?;
                        self.parse_json_slice(&value)
                    })
                    .collect::<Result<Vec<_>, _>>()?;

                Ok((source, parsed_records))
            })
            .collect()
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

fn dynamodb_image<T>(
    image_name: &str,
    index: usize,
    image: serde_dynamo::Item,
) -> Result<ParsedEvent<T>, ParseError>
where
    T: DeserializeOwned,
{
    if image.is_empty() {
        return Err(ParseError::new(
            ParseErrorKind::Data,
            format!("DynamoDB record at index {index} is missing {image_name}"),
        ));
    }

    serde_dynamo::from_item(image)
        .map(ParsedEvent::new)
        .map_err(|error| {
            ParseError::new(
                ParseErrorKind::Data,
                format!("DynamoDB record at index {index} {image_name} cannot be decoded: {error}"),
            )
        })
}

fn kafka_record_value(
    source: &str,
    index: usize,
    record: KafkaRecord,
) -> Result<Vec<u8>, ParseError> {
    let value = record.value.ok_or_else(|| {
        ParseError::new(
            ParseErrorKind::Data,
            format!("Kafka record group {source} at index {index} is missing value"),
        )
    })?;

    base64::engine::general_purpose::STANDARD
        .decode(value)
        .map_err(|error| {
            ParseError::new(
                ParseErrorKind::Data,
                format!(
                    "Kafka record group {source} at index {index} value is not valid base64: {error}"
                ),
            )
        })
}

fn event_body(
    source: &str,
    body: Option<Body>,
    is_base64_encoded: bool,
) -> Result<Vec<u8>, ParseError> {
    let body = body.ok_or_else(|| {
        ParseError::new(
            ParseErrorKind::Data,
            format!("{source} event is missing body"),
        )
    })?;

    if is_base64_encoded {
        let encoded = std::str::from_utf8(body.as_ref()).map_err(|error| {
            ParseError::new(
                ParseErrorKind::Data,
                format!("{source} body is not valid UTF-8 base64 text: {error}"),
            )
        })?;

        base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .map_err(|error| {
                ParseError::new(
                    ParseErrorKind::Data,
                    format!("{source} body is not valid base64: {error}"),
                )
            })
    } else {
        Ok(body.as_ref().to_vec())
    }
}

fn gateway_body(
    source: &str,
    body: Option<String>,
    is_base64_encoded: bool,
) -> Result<Vec<u8>, ParseError> {
    let body = body.ok_or_else(|| {
        ParseError::new(
            ParseErrorKind::Data,
            format!("{source} event is missing body"),
        )
    })?;

    if is_base64_encoded {
        base64::engine::general_purpose::STANDARD
            .decode(body)
            .map_err(|error| {
                ParseError::new(
                    ParseErrorKind::Data,
                    format!("{source} body is not valid base64: {error}"),
                )
            })
    } else {
        Ok(body.into_bytes())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use aws_lambda_events::{
        encodings::Body,
        event::{
            alb::AlbTargetGroupRequest,
            apigw::{
                ApiGatewayProxyRequest, ApiGatewayV2httpRequest, ApiGatewayWebsocketProxyRequest,
            },
            appsync::AppSyncDirectResolverEvent,
            cloudwatch_logs::{LogEntry, LogsEvent},
            dynamodb::Event as DynamoDbEvent,
            eventbridge::EventBridgeEvent,
            firehose::KinesisFirehoseEvent,
            kafka::{KafkaEvent, KafkaRecord},
            kinesis::KinesisEvent,
            lambda_function_urls::LambdaFunctionUrlRequest,
            sns::{SnsEvent, SnsMessage, SnsRecord},
            sqs::{SqsEvent, SqsMessage},
            vpc_lattice::{VpcLatticeRequestV1, VpcLatticeRequestV2},
        },
    };
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    use serde::Deserialize;
    use serde_json::{Value, json};

    use crate::{EventParser, ParseErrorKind};

    #[derive(Debug, Deserialize, Eq, PartialEq)]
    struct OrderEvent {
        order_id: String,
        quantity: u32,
    }

    #[test]
    fn parses_api_gateway_v1_body() {
        let mut event = ApiGatewayProxyRequest::default();
        event.body = Some(r#"{"order_id":"order-1","quantity":2}"#.to_owned());

        let parsed = EventParser::new()
            .parse_apigw_v1_body::<OrderEvent>(event)
            .expect("valid body should parse");

        assert_eq!(parsed.payload().order_id, "order-1");
    }

    #[test]
    fn parses_base64_api_gateway_v2_body() {
        let mut event = ApiGatewayV2httpRequest::default();
        event.body = Some("eyJvcmRlcl9pZCI6Im9yZGVyLTEiLCJxdWFudGl0eSI6Mn0=".to_owned());
        event.is_base64_encoded = true;

        let parsed = EventParser::new()
            .parse_apigw_v2_body::<OrderEvent>(event)
            .expect("valid body should parse");

        assert_eq!(parsed.payload().quantity, 2);
    }

    #[test]
    fn parses_api_gateway_websocket_body() {
        let mut event = ApiGatewayWebsocketProxyRequest::default();
        event.body = Some(r#"{"order_id":"order-1","quantity":2}"#.to_owned());

        let parsed = EventParser::new()
            .parse_apigw_websocket_body::<OrderEvent>(event)
            .expect("valid body should parse");

        assert_eq!(parsed.payload().order_id, "order-1");
    }

    #[test]
    fn parses_appsync_arguments() {
        let mut event = AppSyncDirectResolverEvent::<Value, Value, Value>::default();
        event.arguments = Some(json!({
            "order_id": "order-1",
            "quantity": 2,
        }));

        let parsed = EventParser::new()
            .parse_appsync_arguments::<OrderEvent>(event)
            .expect("valid arguments should parse");

        assert_eq!(parsed.payload().quantity, 2);
    }

    #[test]
    fn parses_appsync_source() {
        let mut event = AppSyncDirectResolverEvent::<Value, Value, Value>::default();
        event.source = Some(json!({
            "order_id": "order-1",
            "quantity": 2,
        }));

        let parsed = EventParser::new()
            .parse_appsync_source::<OrderEvent>(event)
            .expect("valid source should parse");

        assert_eq!(parsed.payload().order_id, "order-1");
    }

    #[test]
    fn parses_alb_body() {
        let mut event = AlbTargetGroupRequest::default();
        event.body = Some(r#"{"order_id":"order-1","quantity":2}"#.to_owned());

        let parsed = EventParser::new()
            .parse_alb_body::<OrderEvent>(event)
            .expect("valid body should parse");

        assert_eq!(parsed.payload().order_id, "order-1");

        let mut event = AlbTargetGroupRequest::default();
        event.body = Some("eyJvcmRlcl9pZCI6Im9yZGVyLTIiLCJxdWFudGl0eSI6M30=".to_owned());
        event.is_base64_encoded = true;

        let parsed = EventParser::new()
            .parse_alb_body::<OrderEvent>(event)
            .expect("valid base64 body should parse");

        assert_eq!(parsed.payload().quantity, 3);
    }

    #[test]
    fn parses_lambda_function_url_body() {
        let mut event = LambdaFunctionUrlRequest::default();
        event.body = Some(r#"{"order_id":"order-1","quantity":2}"#.to_owned());

        let parsed = EventParser::new()
            .parse_lambda_function_url_body::<OrderEvent>(event)
            .expect("valid body should parse");

        assert_eq!(parsed.payload().order_id, "order-1");

        let mut event = LambdaFunctionUrlRequest::default();
        event.body = Some("eyJvcmRlcl9pZCI6Im9yZGVyLTIiLCJxdWFudGl0eSI6M30=".to_owned());
        event.is_base64_encoded = true;

        let parsed = EventParser::new()
            .parse_lambda_function_url_body::<OrderEvent>(event)
            .expect("valid base64 body should parse");

        assert_eq!(parsed.payload().quantity, 3);
    }

    #[test]
    fn parses_vpc_lattice_body() {
        let mut event = VpcLatticeRequestV1::default();
        event.body = Some(Body::from(r#"{"order_id":"order-1","quantity":2}"#));

        let parsed = EventParser::new()
            .parse_vpc_lattice_body::<OrderEvent>(event)
            .expect("valid body should parse");

        assert_eq!(parsed.payload().order_id, "order-1");

        let mut event = VpcLatticeRequestV1::default();
        event.body = Some(Body::from(
            "eyJvcmRlcl9pZCI6Im9yZGVyLTIiLCJxdWFudGl0eSI6M30=",
        ));
        event.is_base64_encoded = true;

        let parsed = EventParser::new()
            .parse_vpc_lattice_body::<OrderEvent>(event)
            .expect("valid base64 body should parse");

        assert_eq!(parsed.payload().quantity, 3);
    }

    #[test]
    fn parses_vpc_lattice_v2_body() {
        let mut event = VpcLatticeRequestV2::default();
        event.body = Some(r#"{"order_id":"order-1","quantity":2}"#.to_owned());

        let parsed = EventParser::new()
            .parse_vpc_lattice_v2_body::<OrderEvent>(event)
            .expect("valid body should parse");

        assert_eq!(parsed.payload().order_id, "order-1");

        let mut event = VpcLatticeRequestV2::default();
        event.body = Some("eyJvcmRlcl9pZCI6Im9yZGVyLTIiLCJxdWFudGl0eSI6M30=".to_owned());
        event.is_base64_encoded = true;

        let parsed = EventParser::new()
            .parse_vpc_lattice_v2_body::<OrderEvent>(event)
            .expect("valid base64 body should parse");

        assert_eq!(parsed.payload().quantity, 3);
    }

    #[test]
    fn rejects_api_gateway_events_without_bodies() {
        let error = EventParser::new()
            .parse_apigw_v1_body::<OrderEvent>(ApiGatewayProxyRequest::default())
            .expect_err("missing body should fail");

        assert_eq!(error.kind(), ParseErrorKind::Data);
        assert_eq!(error.message(), "API Gateway v1 event is missing body");
    }

    #[test]
    fn rejects_appsync_events_without_arguments() {
        let event = AppSyncDirectResolverEvent::<Value, Value, Value>::default();

        let error = EventParser::new()
            .parse_appsync_arguments::<OrderEvent>(event)
            .expect_err("missing arguments should fail");

        assert_eq!(error.kind(), ParseErrorKind::Data);
        assert_eq!(error.message(), "AppSync event is missing arguments");
    }

    #[test]
    fn rejects_invalid_base64_api_gateway_body() {
        let mut event = ApiGatewayV2httpRequest::default();
        event.body = Some("not-base64!".to_owned());
        event.is_base64_encoded = true;

        let error = EventParser::new()
            .parse_apigw_v2_body::<OrderEvent>(event)
            .expect_err("invalid base64 should fail");

        assert_eq!(error.kind(), ParseErrorKind::Data);
        assert!(error.message().contains("not valid base64"));
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
    fn parses_kinesis_record_data() {
        let event: KinesisEvent = serde_json::from_value(json!({
            "Records": [
                {
                    "kinesis": {
                        "kinesisSchemaVersion": "1.0",
                        "partitionKey": "orders",
                        "sequenceNumber": "1",
                        "data": "eyJvcmRlcl9pZCI6Im9yZGVyLTEiLCJxdWFudGl0eSI6Mn0=",
                        "approximateArrivalTimestamp": 1
                    }
                },
                {
                    "kinesis": {
                        "kinesisSchemaVersion": "1.0",
                        "partitionKey": "orders",
                        "sequenceNumber": "2",
                        "data": "eyJvcmRlcl9pZCI6Im9yZGVyLTIiLCJxdWFudGl0eSI6M30=",
                        "approximateArrivalTimestamp": 1
                    }
                }
            ]
        }))
        .expect("kinesis event should deserialize");

        let parsed = EventParser::new()
            .parse_kinesis_records::<OrderEvent>(event)
            .expect("valid records should parse");

        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].payload().order_id, "order-1");
        assert_eq!(parsed[1].payload().quantity, 3);
    }

    #[test]
    fn parses_firehose_record_data() {
        let event: KinesisFirehoseEvent = serde_json::from_value(json!({
            "records": [
                {
                    "recordId": "record-1",
                    "approximateArrivalTimestamp": 1,
                    "data": "eyJvcmRlcl9pZCI6Im9yZGVyLTEiLCJxdWFudGl0eSI6Mn0="
                }
            ]
        }))
        .expect("firehose event should deserialize");

        let parsed = EventParser::new()
            .parse_firehose_records::<OrderEvent>(event)
            .expect("valid records should parse");

        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].payload().order_id, "order-1");
        assert_eq!(parsed[0].payload().quantity, 2);
    }

    #[test]
    fn parses_dynamodb_new_images() {
        let event: DynamoDbEvent = serde_json::from_value(json!({
            "Records": [
                {
                    "eventID": "1",
                    "eventName": "INSERT",
                    "awsRegion": "us-east-1",
                    "eventSource": "aws:dynamodb",
                    "dynamodb": {
                        "ApproximateCreationDateTime": 1,
                        "Keys": {
                            "order_id": {"S": "order-1"}
                        },
                        "NewImage": {
                            "order_id": {"S": "order-1"},
                            "quantity": {"N": "2"}
                        },
                        "SequenceNumber": "1",
                        "SizeBytes": 26,
                        "StreamViewType": "NEW_IMAGE"
                    }
                }
            ]
        }))
        .expect("DynamoDB event should deserialize");

        let parsed = EventParser::new()
            .parse_dynamodb_new_images::<OrderEvent>(event)
            .expect("new image should parse");

        assert_eq!(parsed[0].payload().order_id, "order-1");
        assert_eq!(parsed[0].payload().quantity, 2);
    }

    #[test]
    fn parses_dynamodb_old_images() {
        let event: DynamoDbEvent = serde_json::from_value(json!({
            "Records": [
                {
                    "eventID": "1",
                    "eventName": "MODIFY",
                    "awsRegion": "us-east-1",
                    "eventSource": "aws:dynamodb",
                    "dynamodb": {
                        "ApproximateCreationDateTime": 1,
                        "Keys": {
                            "order_id": {"S": "order-1"}
                        },
                        "OldImage": {
                            "order_id": {"S": "order-1"},
                            "quantity": {"N": "1"}
                        },
                        "NewImage": {
                            "order_id": {"S": "order-1"},
                            "quantity": {"N": "2"}
                        },
                        "SequenceNumber": "1",
                        "SizeBytes": 26,
                        "StreamViewType": "NEW_AND_OLD_IMAGES"
                    }
                }
            ]
        }))
        .expect("DynamoDB event should deserialize");

        let parsed = EventParser::new()
            .parse_dynamodb_old_images::<OrderEvent>(event)
            .expect("old image should parse");

        assert_eq!(parsed[0].payload().order_id, "order-1");
        assert_eq!(parsed[0].payload().quantity, 1);
    }

    #[test]
    fn rejects_dynamodb_missing_new_image() {
        let event: DynamoDbEvent = serde_json::from_value(json!({
            "Records": [
                {
                    "eventID": "1",
                    "eventName": "REMOVE",
                    "awsRegion": "us-east-1",
                    "eventSource": "aws:dynamodb",
                    "dynamodb": {
                        "ApproximateCreationDateTime": 1,
                        "Keys": {
                            "order_id": {"S": "order-1"}
                        },
                        "OldImage": {
                            "order_id": {"S": "order-1"},
                            "quantity": {"N": "1"}
                        },
                        "SequenceNumber": "1",
                        "SizeBytes": 26,
                        "StreamViewType": "OLD_IMAGE"
                    }
                }
            ]
        }))
        .expect("DynamoDB event should deserialize");

        let error = EventParser::new()
            .parse_dynamodb_new_images::<OrderEvent>(event)
            .expect_err("missing new image should fail");

        assert_eq!(error.kind(), ParseErrorKind::Data);
        assert!(error.message().contains("NewImage"));
    }

    #[test]
    fn parses_kafka_record_values() {
        let mut record = KafkaRecord::default();
        record.topic = Some("orders".to_owned());
        record.partition = 0;
        record.offset = 1;
        record.value = Some(STANDARD.encode(r#"{"order_id":"order-1","quantity":2}"#));
        let mut event = KafkaEvent::default();
        event.records = HashMap::from([("orders-0".to_owned(), vec![record])]);

        let parsed = EventParser::new()
            .parse_kafka_record_values::<OrderEvent>(event)
            .expect("record value should parse");

        assert_eq!(parsed["orders-0"][0].payload().order_id, "order-1");
        assert_eq!(parsed["orders-0"][0].payload().quantity, 2);
    }

    #[test]
    fn rejects_kafka_record_without_value() {
        let mut record = KafkaRecord::default();
        record.topic = Some("orders".to_owned());
        record.partition = 0;
        record.offset = 1;
        let mut event = KafkaEvent::default();
        event.records = HashMap::from([("orders-0".to_owned(), vec![record])]);

        let error = EventParser::new()
            .parse_kafka_record_values::<OrderEvent>(event)
            .expect_err("missing value should fail");

        assert_eq!(error.kind(), ParseErrorKind::Data);
        assert!(error.message().contains("missing value"));
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

    #[test]
    fn parses_cloudwatch_log_messages() {
        let mut first = LogEntry::default();
        first.message = r#"{"order_id":"order-1","quantity":2}"#.to_owned();
        let mut second = LogEntry::default();
        second.message = r#"{"order_id":"order-2","quantity":3}"#.to_owned();
        let mut event = LogsEvent::default();
        event.aws_logs.data.log_events = vec![first, second];

        let parsed = EventParser::new()
            .parse_cloudwatch_log_messages::<OrderEvent>(event)
            .expect("valid log messages should parse");

        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].payload().order_id, "order-1");
        assert_eq!(parsed[1].payload().quantity, 3);
    }
}
