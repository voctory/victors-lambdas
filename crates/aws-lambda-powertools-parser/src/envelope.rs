//! Event envelope adapters.

use std::collections::HashMap;

use aws_lambda_events::{
    encodings::Body,
    event::{
        activemq::{ActiveMqEvent, ActiveMqMessage},
        alb::AlbTargetGroupRequest,
        apigw::{ApiGatewayProxyRequest, ApiGatewayV2httpRequest, ApiGatewayWebsocketProxyRequest},
        appsync::AppSyncDirectResolverEvent,
        bedrock_agent_runtime::AgentEvent,
        cloudformation::CloudFormationCustomResourceRequest,
        cloudwatch_logs::LogsEvent,
        cognito::{
            CognitoEventUserPoolsCreateAuthChallenge, CognitoEventUserPoolsCustomMessage,
            CognitoEventUserPoolsDefineAuthChallenge, CognitoEventUserPoolsPostAuthentication,
            CognitoEventUserPoolsPostConfirmation, CognitoEventUserPoolsPreAuthentication,
            CognitoEventUserPoolsPreSignup, CognitoEventUserPoolsPreTokenGen,
            CognitoEventUserPoolsPreTokenGenV2, CognitoEventUserPoolsVerifyAuthChallenge,
        },
        dynamodb::{Event as DynamoDbEvent, EventRecord as DynamoDbEventRecord},
        eventbridge::EventBridgeEvent,
        firehose::KinesisFirehoseEvent,
        kafka::{KafkaEvent, KafkaRecord},
        kinesis::KinesisEvent,
        lambda_function_urls::LambdaFunctionUrlRequest,
        rabbitmq::{RabbitMqEvent, RabbitMqMessage},
        s3::{S3Event, batch_job::S3BatchJobEvent, object_lambda::S3ObjectLambdaEvent},
        ses::SimpleEmailEvent,
        sns::{SnsEvent, SnsMessage},
        sqs::{SqsEvent, SqsMessage},
        vpc_lattice::{VpcLatticeRequestV1, VpcLatticeRequestV2},
    },
};
use base64::Engine;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;

use crate::{
    AppSyncEventsEvent, DynamoDbStreamImageRecord, EventParser, ParseError, ParseErrorKind,
    ParsedEvent, S3EventNotification,
};

impl EventParser {
    /// Parses JSON `ActiveMQ` message data.
    ///
    /// Each `ActiveMQ` message data field is base64-decoded before being decoded
    /// into `T`.
    ///
    /// # Errors
    ///
    /// Returns a parse error when any message data is missing, is not valid
    /// base64, or cannot be decoded into `T`.
    pub fn parse_activemq_message_data<T>(
        &self,
        event: ActiveMqEvent,
    ) -> Result<Vec<ParsedEvent<T>>, ParseError>
    where
        T: DeserializeOwned,
    {
        event
            .messages
            .into_iter()
            .enumerate()
            .map(|(index, message)| {
                let data = activemq_message_data(index, message)?;
                self.parse_json_slice(&data)
            })
            .collect()
    }

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

    /// Parses AWS `AppSync` Events publish payloads.
    ///
    /// Each incoming published event payload is decoded into `T` and returned
    /// in event order.
    ///
    /// # Errors
    ///
    /// Returns a parse error when the `events` collection is missing or any
    /// payload cannot be decoded into `T`.
    pub fn parse_appsync_events_payloads<T>(
        &self,
        event: AppSyncEventsEvent,
    ) -> Result<Vec<ParsedEvent<T>>, ParseError>
    where
        T: DeserializeOwned,
    {
        let events = event.events.ok_or_else(|| {
            ParseError::new(
                ParseErrorKind::Data,
                "AppSync Events event is missing events",
            )
        })?;

        events
            .into_iter()
            .map(|event| self.parse_json_value(event.payload))
            .collect()
    }

    /// Parses the JSON `inputText` payload from a Bedrock Agent event.
    ///
    /// # Errors
    ///
    /// Returns a parse error when `inputText` cannot be decoded into `T`.
    pub fn parse_bedrock_agent_input<T>(
        &self,
        event: AgentEvent,
    ) -> Result<ParsedEvent<T>, ParseError>
    where
        T: DeserializeOwned,
    {
        let input_text = event.input_text;
        self.parse_json_str(&input_text)
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
        let detail = if event.source == "aws.scheduler" && event.detail.as_str() == Some("{}") {
            Value::Object(serde_json::Map::default())
        } else {
            event.detail
        };

        self.parse_json_value(detail)
    }

    /// Parses `CloudFormation` custom resource `ResourceProperties`.
    ///
    /// The current resource properties are decoded into `T` for `Create`,
    /// `Update`, and `Delete` custom resource requests.
    ///
    /// # Errors
    ///
    /// Returns a parse error when `ResourceProperties` cannot be decoded into
    /// `T`.
    pub fn parse_cloudformation_resource_properties<T>(
        &self,
        event: CloudFormationCustomResourceRequest<Value, Value>,
    ) -> Result<ParsedEvent<T>, ParseError>
    where
        T: DeserializeOwned,
    {
        let properties = match event {
            CloudFormationCustomResourceRequest::Create(request) => request.resource_properties,
            CloudFormationCustomResourceRequest::Update(request) => request.resource_properties,
            CloudFormationCustomResourceRequest::Delete(request) => request.resource_properties,
            _ => {
                return Err(ParseError::new(
                    ParseErrorKind::Data,
                    "CloudFormation custom resource request type is not supported",
                ));
            }
        };

        self.parse_json_value(properties)
    }

    /// Parses `CloudFormation` custom resource `OldResourceProperties`.
    ///
    /// # Errors
    ///
    /// Returns a parse error when the request is not an `Update` request or
    /// `OldResourceProperties` cannot be decoded into `T`.
    pub fn parse_cloudformation_old_resource_properties<T>(
        &self,
        event: CloudFormationCustomResourceRequest<Value, Value>,
    ) -> Result<ParsedEvent<T>, ParseError>
    where
        T: DeserializeOwned,
    {
        let CloudFormationCustomResourceRequest::Update(request) = event else {
            return Err(ParseError::new(
                ParseErrorKind::Data,
                "CloudFormation custom resource request is not an Update request",
            ));
        };

        self.parse_json_value(request.old_resource_properties)
    }

    /// Parses Amazon Cognito User Pool Pre sign-up user attributes.
    ///
    /// # Errors
    ///
    /// Returns a parse error when the user attributes cannot be decoded into
    /// `T`.
    pub fn parse_cognito_pre_signup_user_attributes<T>(
        &self,
        event: CognitoEventUserPoolsPreSignup,
    ) -> Result<ParsedEvent<T>, ParseError>
    where
        T: DeserializeOwned,
    {
        parse_serialized_value(
            self,
            "Cognito Pre sign-up user attributes",
            event.request.user_attributes,
        )
    }

    /// Parses Amazon Cognito User Pool Pre authentication user attributes.
    ///
    /// # Errors
    ///
    /// Returns a parse error when the user attributes cannot be decoded into
    /// `T`.
    pub fn parse_cognito_pre_authentication_user_attributes<T>(
        &self,
        event: CognitoEventUserPoolsPreAuthentication,
    ) -> Result<ParsedEvent<T>, ParseError>
    where
        T: DeserializeOwned,
    {
        parse_serialized_value(
            self,
            "Cognito Pre authentication user attributes",
            event.request.user_attributes,
        )
    }

    /// Parses Amazon Cognito User Pool Post confirmation user attributes.
    ///
    /// # Errors
    ///
    /// Returns a parse error when the user attributes cannot be decoded into
    /// `T`.
    pub fn parse_cognito_post_confirmation_user_attributes<T>(
        &self,
        event: CognitoEventUserPoolsPostConfirmation,
    ) -> Result<ParsedEvent<T>, ParseError>
    where
        T: DeserializeOwned,
    {
        parse_serialized_value(
            self,
            "Cognito Post confirmation user attributes",
            event.request.user_attributes,
        )
    }

    /// Parses Amazon Cognito User Pool Pre token generation user attributes.
    ///
    /// # Errors
    ///
    /// Returns a parse error when the user attributes cannot be decoded into
    /// `T`.
    pub fn parse_cognito_pre_token_generation_user_attributes<T>(
        &self,
        event: CognitoEventUserPoolsPreTokenGen,
    ) -> Result<ParsedEvent<T>, ParseError>
    where
        T: DeserializeOwned,
    {
        parse_serialized_value(
            self,
            "Cognito Pre token generation user attributes",
            event.request.user_attributes,
        )
    }

    /// Parses Amazon Cognito User Pool Pre token generation v2/v3 user attributes.
    ///
    /// # Errors
    ///
    /// Returns a parse error when the user attributes cannot be decoded into
    /// `T`.
    pub fn parse_cognito_pre_token_generation_v2_user_attributes<T>(
        &self,
        event: CognitoEventUserPoolsPreTokenGenV2,
    ) -> Result<ParsedEvent<T>, ParseError>
    where
        T: DeserializeOwned,
    {
        parse_serialized_value(
            self,
            "Cognito Pre token generation v2 user attributes",
            event.request.user_attributes,
        )
    }

    /// Parses Amazon Cognito User Pool Post authentication user attributes.
    ///
    /// # Errors
    ///
    /// Returns a parse error when the user attributes cannot be decoded into
    /// `T`.
    pub fn parse_cognito_post_authentication_user_attributes<T>(
        &self,
        event: CognitoEventUserPoolsPostAuthentication,
    ) -> Result<ParsedEvent<T>, ParseError>
    where
        T: DeserializeOwned,
    {
        parse_serialized_value(
            self,
            "Cognito Post authentication user attributes",
            event.request.user_attributes,
        )
    }

    /// Parses Amazon Cognito User Pool Define auth challenge user attributes.
    ///
    /// # Errors
    ///
    /// Returns a parse error when the user attributes cannot be decoded into
    /// `T`.
    pub fn parse_cognito_define_auth_challenge_user_attributes<T>(
        &self,
        event: CognitoEventUserPoolsDefineAuthChallenge,
    ) -> Result<ParsedEvent<T>, ParseError>
    where
        T: DeserializeOwned,
    {
        parse_serialized_value(
            self,
            "Cognito Define auth challenge user attributes",
            event.request.user_attributes,
        )
    }

    /// Parses Amazon Cognito User Pool Create auth challenge user attributes.
    ///
    /// # Errors
    ///
    /// Returns a parse error when the user attributes cannot be decoded into
    /// `T`.
    pub fn parse_cognito_create_auth_challenge_user_attributes<T>(
        &self,
        event: CognitoEventUserPoolsCreateAuthChallenge,
    ) -> Result<ParsedEvent<T>, ParseError>
    where
        T: DeserializeOwned,
    {
        parse_serialized_value(
            self,
            "Cognito Create auth challenge user attributes",
            event.request.user_attributes,
        )
    }

    /// Parses Amazon Cognito User Pool Verify auth challenge user attributes.
    ///
    /// # Errors
    ///
    /// Returns a parse error when the user attributes cannot be decoded into
    /// `T`.
    pub fn parse_cognito_verify_auth_challenge_user_attributes<T>(
        &self,
        event: CognitoEventUserPoolsVerifyAuthChallenge,
    ) -> Result<ParsedEvent<T>, ParseError>
    where
        T: DeserializeOwned,
    {
        parse_serialized_value(
            self,
            "Cognito Verify auth challenge user attributes",
            event.request.user_attributes,
        )
    }

    /// Parses Amazon Cognito User Pool Custom message user attributes.
    ///
    /// # Errors
    ///
    /// Returns a parse error when the user attributes cannot be decoded into
    /// `T`.
    pub fn parse_cognito_custom_message_user_attributes<T>(
        &self,
        event: CognitoEventUserPoolsCustomMessage,
    ) -> Result<ParsedEvent<T>, ParseError>
    where
        T: DeserializeOwned,
    {
        parse_serialized_value(
            self,
            "Cognito Custom message user attributes",
            event.request.user_attributes,
        )
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

    /// Parses JSON `CloudWatch Logs` messages delivered through Kinesis data.
    ///
    /// Each Kinesis record data blob must contain the compressed `CloudWatch
    /// Logs` subscription payload. Log event messages are decoded into `T` and
    /// returned in Kinesis record order, preserving log event order within each
    /// record.
    ///
    /// # Errors
    ///
    /// Returns a parse error when any Kinesis record data is not a valid
    /// `CloudWatch Logs` payload or any log event message cannot be decoded
    /// into `T`.
    pub fn parse_kinesis_cloudwatch_log_messages<T>(
        &self,
        event: KinesisEvent,
    ) -> Result<Vec<ParsedEvent<T>>, ParseError>
    where
        T: DeserializeOwned,
    {
        let mut parsed = Vec::new();

        for (record_index, record) in event.records.into_iter().enumerate() {
            let logs = parse_kinesis_cloudwatch_logs(record_index, &record.kinesis.data)?;

            for (message_index, entry) in logs.aws_logs.data.log_events.into_iter().enumerate() {
                let message = self.parse_json_str(&entry.message).map_err(|error| {
                    ParseError::new(
                        error.kind(),
                        format!(
                            "Kinesis record at index {record_index} CloudWatch Logs message at index {message_index} is not a valid payload: {}",
                            error.message()
                        ),
                    )
                })?;
                parsed.push(message);
            }
        }

        Ok(parsed)
    }

    /// Parses `DynamoDB` stream `NewImage` records delivered through Kinesis data.
    ///
    /// Each Kinesis record data blob must contain a JSON `DynamoDB` stream
    /// record. The nested non-empty `NewImage` item is decoded into `T` with
    /// `serde_dynamo` and returned in Kinesis record order.
    ///
    /// # Errors
    ///
    /// Returns a parse error when Kinesis record data is not a valid `DynamoDB`
    /// stream record, any record is missing a `NewImage`, or an image cannot be
    /// decoded into `T`.
    pub fn parse_kinesis_dynamodb_new_images<T>(
        &self,
        event: KinesisEvent,
    ) -> Result<Vec<ParsedEvent<T>>, ParseError>
    where
        T: DeserializeOwned,
    {
        event
            .records
            .into_iter()
            .enumerate()
            .map(|(index, record)| {
                let record = parse_kinesis_record_json(index, &record.kinesis.data)?;
                dynamodb_image("NewImage", index, record.change.new_image)
            })
            .collect()
    }

    /// Parses `DynamoDB` stream `OldImage` records delivered through Kinesis data.
    ///
    /// Each Kinesis record data blob must contain a JSON `DynamoDB` stream
    /// record. The nested non-empty `OldImage` item is decoded into `T` with
    /// `serde_dynamo` and returned in Kinesis record order.
    ///
    /// # Errors
    ///
    /// Returns a parse error when Kinesis record data is not a valid `DynamoDB`
    /// stream record, any record is missing an `OldImage`, or an image cannot
    /// be decoded into `T`.
    pub fn parse_kinesis_dynamodb_old_images<T>(
        &self,
        event: KinesisEvent,
    ) -> Result<Vec<ParsedEvent<T>>, ParseError>
    where
        T: DeserializeOwned,
    {
        event
            .records
            .into_iter()
            .enumerate()
            .map(|(index, record)| {
                let record = parse_kinesis_record_json(index, &record.kinesis.data)?;
                dynamodb_image("OldImage", index, record.change.old_image)
            })
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

    /// Parses JSON SQS message bodies delivered through Kinesis Firehose data.
    ///
    /// Each Firehose record data blob must contain a JSON SQS message. The SQS
    /// message body is decoded into `T` and returned in Firehose record order.
    ///
    /// # Errors
    ///
    /// Returns a parse error when Firehose record data is not a valid SQS
    /// message, an embedded SQS message is missing a body, or any body cannot
    /// be decoded into `T`.
    pub fn parse_firehose_sqs_message_bodies<T>(
        &self,
        event: KinesisFirehoseEvent,
    ) -> Result<Vec<ParsedEvent<T>>, ParseError>
    where
        T: DeserializeOwned,
    {
        event
            .records
            .into_iter()
            .enumerate()
            .map(|(index, record)| {
                let message: SqsMessage =
                    parse_firehose_record_json("SQS message", index, &record.data)?;
                let body = sqs_body(index, message.body)?;
                self.parse_json_str(&body)
            })
            .collect()
    }

    /// Parses `DynamoDB` stream `NewImage` and `OldImage` records.
    ///
    /// Each non-empty image item is decoded into `T` with `serde_dynamo`.
    /// Missing images are returned as `None`, preserving the original stream
    /// record order and image presence.
    ///
    /// # Errors
    ///
    /// Returns a parse error when any present image cannot be decoded into `T`.
    pub fn parse_dynamodb_images<T>(
        &self,
        event: DynamoDbEvent,
    ) -> Result<Vec<DynamoDbStreamImageRecord<T>>, ParseError>
    where
        T: DeserializeOwned,
    {
        event
            .records
            .into_iter()
            .enumerate()
            .map(|(index, record)| {
                let new_image =
                    optional_dynamodb_image("NewImage", index, record.change.new_image)?;
                let old_image =
                    optional_dynamodb_image("OldImage", index, record.change.old_image)?;
                Ok(DynamoDbStreamImageRecord::new(new_image, old_image))
            })
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

    /// Parses JSON `RabbitMQ` message data.
    ///
    /// `RabbitMQ` messages are returned with the same queue grouping used by the
    /// Lambda event. Each message data field is base64-decoded before being
    /// decoded into `T`.
    ///
    /// # Errors
    ///
    /// Returns a parse error when any message data is missing, is not valid
    /// base64, or cannot be decoded into `T`.
    pub fn parse_rabbitmq_message_data<T>(
        &self,
        event: RabbitMqEvent,
    ) -> Result<HashMap<String, Vec<ParsedEvent<T>>>, ParseError>
    where
        T: DeserializeOwned,
    {
        event
            .messages_by_queue
            .into_iter()
            .map(|(queue, messages)| {
                let parsed_messages = messages
                    .into_iter()
                    .enumerate()
                    .map(|(index, message)| {
                        let data = rabbitmq_message_data(&queue, index, message)?;
                        self.parse_json_slice(&data)
                    })
                    .collect::<Result<Vec<_>, _>>()?;

                Ok((queue, parsed_messages))
            })
            .collect()
    }

    /// Parses Amazon S3 event records.
    ///
    /// Each S3 event record is decoded into `T` and returned in record order.
    /// This is most useful with a target type matching the S3 record shape from
    /// `aws_lambda_events::event::s3`.
    ///
    /// # Errors
    ///
    /// Returns a parse error when any S3 record cannot be decoded into `T`.
    pub fn parse_s3_records<T>(&self, event: S3Event) -> Result<Vec<ParsedEvent<T>>, ParseError>
    where
        T: DeserializeOwned,
    {
        event
            .records
            .into_iter()
            .enumerate()
            .map(|(index, record)| parse_record_value(self, "S3", index, record))
            .collect()
    }

    /// Parses owned Amazon S3 event notification records.
    ///
    /// Use this model when events may include notification variants not covered
    /// by `aws_lambda_events`, such as S3 Intelligent-Tiering records that use
    /// `s3.get_object` instead of `s3.object`.
    ///
    /// # Errors
    ///
    /// Returns a parse error when any S3 notification record cannot be decoded
    /// into `T`.
    pub fn parse_s3_event_notification_records<T>(
        &self,
        event: S3EventNotification,
    ) -> Result<Vec<ParsedEvent<T>>, ParseError>
    where
        T: DeserializeOwned,
    {
        event
            .records
            .into_iter()
            .enumerate()
            .map(|(index, record)| parse_record_value(self, "S3 event notification", index, record))
            .collect()
    }

    /// Parses an Amazon S3 Object Lambda configuration payload.
    ///
    /// # Errors
    ///
    /// Returns a parse error when the Object Lambda configuration payload
    /// cannot be decoded into `T`.
    pub fn parse_s3_object_lambda_configuration_payload<T>(
        &self,
        event: S3ObjectLambdaEvent<Value>,
    ) -> Result<ParsedEvent<T>, ParseError>
    where
        T: DeserializeOwned,
    {
        self.parse_json_value(event.configuration.payload)
    }

    /// Parses Amazon S3 Batch job tasks.
    ///
    /// Each S3 Batch task is decoded into `T` and returned in task order. This
    /// is most useful with a target type matching the S3 Batch task shape from
    /// `aws_lambda_events::event::s3::batch_job`.
    ///
    /// # Errors
    ///
    /// Returns a parse error when any S3 Batch task cannot be decoded into `T`.
    pub fn parse_s3_batch_job_tasks<T>(
        &self,
        event: S3BatchJobEvent,
    ) -> Result<Vec<ParsedEvent<T>>, ParseError>
    where
        T: DeserializeOwned,
    {
        event
            .tasks
            .into_iter()
            .enumerate()
            .map(|(index, task)| parse_record_value(self, "S3 Batch job task", index, task))
            .collect()
    }

    /// Parses Amazon S3 event records delivered through SQS message bodies.
    ///
    /// Each SQS record body must contain a JSON S3 event notification. Inner S3
    /// records are flattened and returned in S3 record order for each SQS
    /// message.
    ///
    /// # Errors
    ///
    /// Returns a parse error when an SQS record is missing a body, a body is
    /// not a valid S3 event notification, or an S3 record cannot be decoded
    /// into `T`.
    pub fn parse_s3_sqs_event_records<T>(
        &self,
        event: SqsEvent,
    ) -> Result<Vec<ParsedEvent<T>>, ParseError>
    where
        T: DeserializeOwned,
    {
        let mut parsed = Vec::new();

        for (index, record) in event.records.into_iter().enumerate() {
            let body = sqs_body(index, record.body)?;
            let s3_event: S3Event = parse_sqs_nested_json("S3 event notification", index, &body)?;
            parsed.extend(self.parse_s3_records(s3_event)?);
        }

        Ok(parsed)
    }

    /// Parses owned Amazon S3 event notification records delivered through SQS
    /// message bodies.
    ///
    /// Use this variant when nested S3 notifications may include records not
    /// covered by `aws_lambda_events`, such as S3 Intelligent-Tiering records.
    ///
    /// # Errors
    ///
    /// Returns a parse error when an SQS record is missing a body, a body is
    /// not a valid S3 event notification, or an S3 record cannot be decoded
    /// into `T`.
    pub fn parse_s3_sqs_event_notification_records<T>(
        &self,
        event: SqsEvent,
    ) -> Result<Vec<ParsedEvent<T>>, ParseError>
    where
        T: DeserializeOwned,
    {
        let mut parsed = Vec::new();

        for (index, record) in event.records.into_iter().enumerate() {
            let body = sqs_body(index, record.body)?;
            let s3_event: S3EventNotification =
                parse_sqs_nested_json("S3 event notification", index, &body)?;
            parsed.extend(self.parse_s3_event_notification_records(s3_event)?);
        }

        Ok(parsed)
    }

    /// Parses Amazon SES event records.
    ///
    /// Each SES record is decoded into `T` and returned in record order. This
    /// is most useful with a target type matching the SES record shape from
    /// `aws_lambda_events::event::ses`.
    ///
    /// # Errors
    ///
    /// Returns a parse error when any SES record cannot be decoded into `T`.
    pub fn parse_ses_records<T>(
        &self,
        event: SimpleEmailEvent,
    ) -> Result<Vec<ParsedEvent<T>>, ParseError>
    where
        T: DeserializeOwned,
    {
        event
            .records
            .into_iter()
            .enumerate()
            .map(|(index, record)| parse_record_value(self, "SES", index, record))
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
                let body = sqs_body(index, record.body)?;
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

    /// Parses JSON SNS notification messages delivered through SQS bodies.
    ///
    /// Each SQS record body must contain a JSON SNS notification. The SNS
    /// notification `Message` field is decoded into `T` and returned in SQS
    /// record order.
    ///
    /// # Errors
    ///
    /// Returns a parse error when an SQS record is missing a body, a body is
    /// not a valid SNS notification, or an SNS notification message cannot be
    /// decoded into `T`.
    pub fn parse_sns_sqs_messages<T>(
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
                let body = sqs_body(index, record.body)?;
                let notification: SnsMessage =
                    parse_sqs_nested_json("SNS notification", index, &body)?;
                self.parse_json_str(&notification.message)
            })
            .collect()
    }
}

fn parse_record_value<T, U>(
    parser: &EventParser,
    source: &str,
    index: usize,
    record: U,
) -> Result<ParsedEvent<T>, ParseError>
where
    T: DeserializeOwned,
    U: Serialize,
{
    let value = serde_json::to_value(record).map_err(|error| {
        ParseError::new(
            ParseErrorKind::Data,
            format!("{source} record at index {index} cannot be encoded as JSON: {error}"),
        )
    })?;

    parser.parse_json_value(value)
}

fn parse_serialized_value<T, U>(
    parser: &EventParser,
    source: &str,
    value: U,
) -> Result<ParsedEvent<T>, ParseError>
where
    T: DeserializeOwned,
    U: Serialize,
{
    let value = serde_json::to_value(value).map_err(|error| {
        ParseError::new(
            ParseErrorKind::Data,
            format!("failed to serialize {source}: {error}"),
        )
    })?;

    parser.parse_json_value(value)
}

fn parse_sqs_nested_json<T>(source: &str, index: usize, body: &str) -> Result<T, ParseError>
where
    T: DeserializeOwned,
{
    serde_json::from_str(body).map_err(|error| {
        let error = ParseError::from_json_error(&error);
        ParseError::new(
            error.kind(),
            format!(
                "SQS record at index {index} body is not a valid {source}: {}",
                error.message()
            ),
        )
    })
}

fn parse_firehose_record_json<T>(source: &str, index: usize, data: &[u8]) -> Result<T, ParseError>
where
    T: DeserializeOwned,
{
    serde_json::from_slice(data).map_err(|error| {
        let error = ParseError::from_json_error(&error);
        ParseError::new(
            error.kind(),
            format!(
                "Firehose record at index {index} data is not a valid {source}: {}",
                error.message()
            ),
        )
    })
}

fn parse_kinesis_record_json(index: usize, data: &[u8]) -> Result<DynamoDbEventRecord, ParseError> {
    serde_json::from_slice(data).map_err(|error| {
        let error = ParseError::from_json_error(&error);
        ParseError::new(
            error.kind(),
            format!(
                "Kinesis record at index {index} data is not a valid DynamoDB stream record: {}",
                error.message()
            ),
        )
    })
}

fn parse_kinesis_cloudwatch_logs(index: usize, data: &[u8]) -> Result<LogsEvent, ParseError> {
    let encoded = base64::engine::general_purpose::STANDARD.encode(data);
    let value = format!(r#"{{"awslogs":{{"data":"{encoded}"}}}}"#);

    serde_json::from_str(&value).map_err(|error| {
        let error = ParseError::from_json_error(&error);
        ParseError::new(
            error.kind(),
            format!(
                "Kinesis record at index {index} data is not a valid CloudWatch Logs payload: {}",
                error.message()
            ),
        )
    })
}

fn sqs_body(index: usize, body: Option<String>) -> Result<String, ParseError> {
    body.ok_or_else(|| {
        ParseError::new(
            ParseErrorKind::Data,
            format!("SQS record at index {index} is missing body"),
        )
    })
}

fn optional_dynamodb_image<T>(
    image_name: &str,
    index: usize,
    image: serde_dynamo::Item,
) -> Result<Option<ParsedEvent<T>>, ParseError>
where
    T: DeserializeOwned,
{
    if image.is_empty() {
        return Ok(None);
    }

    dynamodb_image(image_name, index, image).map(Some)
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

fn activemq_message_data(index: usize, message: ActiveMqMessage) -> Result<Vec<u8>, ParseError> {
    decode_mq_data("ActiveMQ", None, index, message.data)
}

fn rabbitmq_message_data(
    queue: &str,
    index: usize,
    message: RabbitMqMessage,
) -> Result<Vec<u8>, ParseError> {
    decode_mq_data("RabbitMQ", Some(queue), index, message.data)
}

fn decode_mq_data(
    source: &str,
    group: Option<&str>,
    index: usize,
    data: Option<String>,
) -> Result<Vec<u8>, ParseError> {
    let location = match group {
        Some(group) => format!("{source} message group {group} at index {index}"),
        None => format!("{source} message at index {index}"),
    };
    let data = data.ok_or_else(|| {
        ParseError::new(ParseErrorKind::Data, format!("{location} is missing data"))
    })?;

    base64::engine::general_purpose::STANDARD
        .decode(data)
        .map_err(|error| {
            ParseError::new(
                ParseErrorKind::Data,
                format!("{location} data is not valid base64: {error}"),
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
            activemq::ActiveMqEvent,
            alb::AlbTargetGroupRequest,
            apigw::{
                ApiGatewayProxyRequest, ApiGatewayV2httpRequest, ApiGatewayWebsocketProxyRequest,
            },
            appsync::AppSyncDirectResolverEvent,
            bedrock_agent_runtime::AgentEvent,
            cloudformation::CloudFormationCustomResourceRequest,
            cloudwatch_logs::{LogEntry, LogsEvent},
            cognito::{
                CognitoEventUserPoolsCreateAuthChallenge, CognitoEventUserPoolsCustomMessage,
                CognitoEventUserPoolsDefineAuthChallenge, CognitoEventUserPoolsPostAuthentication,
                CognitoEventUserPoolsPostConfirmation, CognitoEventUserPoolsPreAuthentication,
                CognitoEventUserPoolsPreSignup, CognitoEventUserPoolsPreTokenGen,
                CognitoEventUserPoolsPreTokenGenV2, CognitoEventUserPoolsVerifyAuthChallenge,
            },
            dynamodb::Event as DynamoDbEvent,
            eventbridge::EventBridgeEvent,
            firehose::KinesisFirehoseEvent,
            kafka::{KafkaEvent, KafkaRecord},
            kinesis::KinesisEvent,
            lambda_function_urls::LambdaFunctionUrlRequest,
            rabbitmq::RabbitMqEvent,
            s3::{S3Event, batch_job::S3BatchJobEvent, object_lambda::S3ObjectLambdaEvent},
            ses::SimpleEmailEvent,
            sns::{SnsEvent, SnsMessage, SnsRecord},
            sqs::{SqsEvent, SqsMessage},
            vpc_lattice::{VpcLatticeRequestV1, VpcLatticeRequestV2},
        },
    };
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    use serde::Deserialize;
    use serde_json::{Value, json};

    use crate::{AppSyncEventsEvent, EventParser, ParseErrorKind};

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
        email: String,
        name: String,
    }

    #[derive(Debug, Deserialize, Eq, PartialEq)]
    #[serde(rename_all = "camelCase")]
    struct S3RecordSummary {
        event_name: Option<String>,
        s3: S3RecordEntity,
    }

    #[derive(Debug, Deserialize, Eq, PartialEq)]
    struct S3RecordEntity {
        bucket: S3RecordBucket,
        object: S3RecordObject,
    }

    #[derive(Debug, Deserialize, Eq, PartialEq)]
    #[serde(rename_all = "camelCase")]
    struct S3NotificationRecordSummary {
        event_name: String,
        s3: S3NotificationRecordEntity,
        intelligent_tiering_event_data: Option<S3NotificationIntelligentTieringData>,
    }

    #[derive(Debug, Deserialize, Eq, PartialEq)]
    struct S3NotificationRecordEntity {
        bucket: S3RecordBucket,
        #[serde(rename = "get_object")]
        get_object: Option<S3RecordObject>,
    }

    #[derive(Debug, Deserialize, Eq, PartialEq)]
    #[serde(rename_all = "camelCase")]
    struct S3NotificationIntelligentTieringData {
        destination_access_tier: String,
    }

    #[derive(Debug, Deserialize, Eq, PartialEq)]
    struct S3RecordBucket {
        name: Option<String>,
    }

    #[derive(Debug, Deserialize, Eq, PartialEq)]
    struct S3RecordObject {
        key: Option<String>,
        size: Option<i64>,
    }

    #[derive(Debug, Deserialize, Eq, PartialEq)]
    #[serde(rename_all = "camelCase")]
    struct S3ObjectLambdaPayload {
        tenant: String,
        redact: bool,
    }

    #[derive(Debug, Deserialize, Eq, PartialEq)]
    #[serde(rename_all = "camelCase")]
    struct S3BatchTaskSummary {
        task_id: Option<String>,
        s3_key: Option<String>,
        s3_bucket_arn: Option<String>,
    }

    #[derive(Debug, Deserialize, Eq, PartialEq)]
    #[serde(rename_all = "camelCase")]
    struct SesRecordSummary {
        event_source: Option<String>,
        ses: SesRecordService,
    }

    #[derive(Debug, Deserialize, Eq, PartialEq)]
    #[serde(rename_all = "camelCase")]
    struct SesRecordService {
        mail: SesRecordMail,
        receipt: SesRecordReceipt,
    }

    #[derive(Debug, Deserialize, Eq, PartialEq)]
    #[serde(rename_all = "camelCase")]
    struct SesRecordMail {
        message_id: Option<String>,
        common_headers: SesRecordCommonHeaders,
    }

    #[derive(Debug, Deserialize, Eq, PartialEq)]
    #[serde(rename_all = "camelCase")]
    struct SesRecordCommonHeaders {
        subject: Option<String>,
    }

    #[derive(Debug, Deserialize, Eq, PartialEq)]
    #[serde(rename_all = "camelCase")]
    struct SesRecordReceipt {
        action: SesRecordAction,
    }

    #[derive(Debug, Deserialize, Eq, PartialEq)]
    struct SesRecordAction {
        #[serde(rename = "type")]
        type_: Option<String>,
    }

    fn cognito_event(trigger_source: &str, request: Value, response: Value) -> Value {
        let mut event = json!({
            "version": "1",
            "triggerSource": trigger_source,
            "region": "us-east-1",
            "userPoolId": "us-east-1_ABC123",
            "userName": "test-user",
            "callerContext": {
                "awsSdkVersion": "2.814.0",
                "clientId": "client123"
            },
            "request": {},
            "response": {}
        });
        event["request"] = request;
        event["response"] = response;
        event
    }

    fn cognito_user_attributes() -> Value {
        json!({
            "email": "user@example.com",
            "name": "Test User"
        })
    }

    fn cognito_group_configuration() -> Value {
        json!({
            "groupsToOverride": ["users"],
            "iamRolesToOverride": [],
            "preferredRole": null
        })
    }

    #[test]
    fn parses_activemq_message_data() {
        let event: ActiveMqEvent = serde_json::from_value(json!({
            "eventSource": "aws:amazonmq",
            "eventSourceArn": "arn:aws:amazonmq:us-east-1:123456789012:broker:orders:b-1",
            "messages": [
                {
                    "messageID": "message-active-mq-1",
                    "messageType": "text",
                    "timestamp": 1_767_225_600_000_i64,
                    "deliveryMode": 2,
                    "correlationID": "correlation-1",
                    "destination": {
                        "physicalName": "orders"
                    },
                    "redelivered": false,
                    "type": "OrderCreated",
                    "expiration": 0,
                    "priority": 4,
                    "data": "eyJvcmRlcl9pZCI6Im9yZGVyLWFjdGl2ZS1tcS0xIiwicXVhbnRpdHkiOjI2fQ==",
                    "brokerInTime": 1_767_225_600_000_i64,
                    "brokerOutTime": 1_767_225_600_001_i64,
                    "properties": {}
                }
            ]
        }))
        .expect("ActiveMQ event should deserialize");

        let parsed = EventParser::new()
            .parse_activemq_message_data::<OrderEvent>(event)
            .expect("ActiveMQ message data should parse");

        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].payload().order_id, "order-active-mq-1");
        assert_eq!(parsed[0].payload().quantity, 26);
    }

    #[test]
    fn parses_rabbitmq_message_data() {
        let event: RabbitMqEvent = serde_json::from_value(json!({
            "eventSource": "aws:amazonmq",
            "eventSourceArn": "arn:aws:amazonmq:us-east-1:123456789012:broker:orders:b-1",
            "rmqMessagesByQueue": {
                "orders::/": [
                    {
                        "basicProperties": {
                            "contentType": "application/json",
                            "contentEncoding": "utf-8",
                            "headers": {},
                            "deliveryMode": 2,
                            "priority": 0,
                            "correlationId": "correlation-1",
                            "messageId": "message-rabbit-mq-1",
                            "timestamp": "2026-01-01T00:00:00Z",
                            "type": "OrderCreated",
                            "appId": "checkout",
                            "bodySize": 47
                        },
                        "data": "eyJvcmRlcl9pZCI6Im9yZGVyLXJhYmJpdC1tcS0xIiwicXVhbnRpdHkiOjI3fQ==",
                        "redelivered": false
                    }
                ]
            }
        }))
        .expect("RabbitMQ event should deserialize");

        let parsed = EventParser::new()
            .parse_rabbitmq_message_data::<OrderEvent>(event)
            .expect("RabbitMQ message data should parse");

        assert_eq!(parsed["orders::/"].len(), 1);
        assert_eq!(
            parsed["orders::/"][0].payload().order_id,
            "order-rabbit-mq-1"
        );
        assert_eq!(parsed["orders::/"][0].payload().quantity, 27);
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
    fn parses_appsync_events_payloads() {
        let event = serde_json::from_value::<AppSyncEventsEvent>(json!({
            "identity": null,
            "request": {
                "headers": {
                    "header1": "value1"
                },
                "domainName": "events.example.com"
            },
            "info": {
                "channel": {
                    "path": "/default/orders",
                    "segments": ["default", "orders"]
                },
                "channelNamespace": {
                    "name": "default"
                },
                "operation": "PUBLISH"
            },
            "stash": {},
            "events": [
                {
                    "payload": {
                        "order_id": "order-1",
                        "quantity": 2
                    },
                    "id": "event-1"
                },
                {
                    "payload": {
                        "order_id": "order-2",
                        "quantity": 3
                    },
                    "id": "event-2"
                }
            ]
        }))
        .expect("AppSync Events event should deserialize");

        let parsed = EventParser::new()
            .parse_appsync_events_payloads::<OrderEvent>(event)
            .expect("valid payloads should parse");

        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].payload().order_id, "order-1");
        assert_eq!(parsed[1].payload().quantity, 3);
    }

    #[test]
    fn parses_bedrock_agent_input() {
        let mut event = AgentEvent::default();
        event.input_text = r#"{"order_id":"order-1","quantity":2}"#.to_owned();

        let parsed = EventParser::new()
            .parse_bedrock_agent_input::<OrderEvent>(event)
            .expect("valid input text should parse");

        assert_eq!(parsed.payload().quantity, 2);
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
    fn rejects_appsync_events_without_publish_payloads() {
        let event = serde_json::from_value::<AppSyncEventsEvent>(json!({
            "identity": null,
            "request": {
                "headers": {},
                "domainName": null
            },
            "info": {
                "channel": {
                    "path": "/default/orders",
                    "segments": ["default", "orders"]
                },
                "channelNamespace": {
                    "name": "default"
                },
                "operation": "SUBSCRIBE"
            },
            "stash": {},
            "events": null
        }))
        .expect("AppSync Events event should deserialize");

        let error = EventParser::new()
            .parse_appsync_events_payloads::<OrderEvent>(event)
            .expect_err("missing publish events should fail");

        assert_eq!(error.kind(), ParseErrorKind::Data);
        assert_eq!(error.message(), "AppSync Events event is missing events");
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
    fn parses_eventbridge_scheduler_empty_detail_string() {
        let mut event = EventBridgeEvent::<Value>::default();
        event.detail_type = "Scheduled Event".to_owned();
        event.source = "aws.scheduler".to_owned();
        event.detail = Value::String("{}".to_owned());

        let parsed = EventParser::new()
            .parse_eventbridge_detail::<Value>(event)
            .expect("Scheduler empty detail should parse as an object");

        assert_eq!(parsed.into_payload(), json!({}));
    }

    #[test]
    fn parses_cloudformation_resource_properties() {
        let event: CloudFormationCustomResourceRequest<Value, Value> =
            serde_json::from_value(json!({
                "RequestType": "Create",
                "ServiceToken": "arn:aws:lambda:us-east-1:123456789012:function:handler",
                "RequestId": "request-1",
                "ResponseURL": "https://example.com/response",
                "StackId": "arn:aws:cloudformation:us-east-1:123456789012:stack/test/1",
                "ResourceType": "Custom::BucketPolicy",
                "LogicalResourceId": "BucketPolicy",
                "ResourceProperties": {
                    "BucketName": "orders",
                    "RetentionDays": 30
                }
            }))
            .expect("CloudFormation request should deserialize");

        let parsed = EventParser::new()
            .parse_cloudformation_resource_properties::<CustomResourceProperties>(event)
            .expect("resource properties should parse");

        assert_eq!(parsed.payload().bucket_name, "orders");
        assert_eq!(parsed.payload().retention_days, 30);
    }

    #[test]
    fn parses_cloudformation_old_resource_properties() {
        let event: CloudFormationCustomResourceRequest<Value, Value> =
            serde_json::from_value(json!({
                "RequestType": "Update",
                "ServiceToken": "arn:aws:lambda:us-east-1:123456789012:function:handler",
                "RequestId": "request-1",
                "ResponseURL": "https://example.com/response",
                "StackId": "arn:aws:cloudformation:us-east-1:123456789012:stack/test/1",
                "ResourceType": "Custom::BucketPolicy",
                "LogicalResourceId": "BucketPolicy",
                "PhysicalResourceId": "bucket-policy-1",
                "ResourceProperties": {
                    "BucketName": "orders",
                    "RetentionDays": 30
                },
                "OldResourceProperties": {
                    "BucketName": "orders",
                    "RetentionDays": 7
                }
            }))
            .expect("CloudFormation request should deserialize");

        let parsed = EventParser::new()
            .parse_cloudformation_old_resource_properties::<CustomResourceProperties>(event)
            .expect("old resource properties should parse");

        assert_eq!(parsed.payload().bucket_name, "orders");
        assert_eq!(parsed.payload().retention_days, 7);
    }

    #[test]
    fn rejects_cloudformation_old_resource_properties_for_create() {
        let event: CloudFormationCustomResourceRequest<Value, Value> =
            serde_json::from_value(json!({
                "RequestType": "Create",
                "ServiceToken": "arn:aws:lambda:us-east-1:123456789012:function:handler",
                "RequestId": "request-1",
                "ResponseURL": "https://example.com/response",
                "StackId": "arn:aws:cloudformation:us-east-1:123456789012:stack/test/1",
                "ResourceType": "Custom::BucketPolicy",
                "LogicalResourceId": "BucketPolicy",
                "ResourceProperties": {
                    "BucketName": "orders",
                    "RetentionDays": 30
                }
            }))
            .expect("CloudFormation request should deserialize");

        let error = EventParser::new()
            .parse_cloudformation_old_resource_properties::<CustomResourceProperties>(event)
            .expect_err("create request should fail");

        assert_eq!(error.kind(), ParseErrorKind::Data);
        assert!(error.message().contains("not an Update request"));
    }

    #[test]
    fn parses_cognito_pre_signup_user_attributes() {
        let event: CognitoEventUserPoolsPreSignup = serde_json::from_value(cognito_event(
            "PreSignUp_SignUp",
            json!({
                "userAttributes": cognito_user_attributes(),
                "validationData": null,
                "clientMetadata": {
                    "source": "test"
                }
            }),
            json!({
                "autoConfirmUser": false,
                "autoVerifyEmail": false,
                "autoVerifyPhone": false
            }),
        ))
        .expect("Cognito Pre sign-up event should deserialize");

        let parsed = EventParser::new()
            .parse_cognito_pre_signup_user_attributes::<CognitoUserAttributes>(event)
            .expect("Cognito user attributes should parse");

        assert_eq!(parsed.payload().email, "user@example.com");
    }

    #[test]
    fn parses_cognito_pre_authentication_user_attributes() {
        let event: CognitoEventUserPoolsPreAuthentication = serde_json::from_value(cognito_event(
            "PreAuthentication_Authentication",
            json!({
                "userAttributes": cognito_user_attributes(),
                "validationData": {
                    "source": "test"
                }
            }),
            json!({}),
        ))
        .expect("Cognito Pre authentication event should deserialize");

        let parsed = EventParser::new()
            .parse_cognito_pre_authentication_user_attributes::<CognitoUserAttributes>(event)
            .expect("Cognito user attributes should parse");

        assert_eq!(parsed.payload().name, "Test User");
    }

    #[test]
    fn parses_cognito_post_confirmation_user_attributes() {
        let event: CognitoEventUserPoolsPostConfirmation = serde_json::from_value(cognito_event(
            "PostConfirmation_ConfirmSignUp",
            json!({
                "userAttributes": cognito_user_attributes(),
                "clientMetadata": {
                    "source": "test"
                }
            }),
            json!({}),
        ))
        .expect("Cognito Post confirmation event should deserialize");

        let parsed = EventParser::new()
            .parse_cognito_post_confirmation_user_attributes::<CognitoUserAttributes>(event)
            .expect("Cognito user attributes should parse");

        assert_eq!(parsed.payload().email, "user@example.com");
    }

    #[test]
    fn parses_cognito_pre_token_generation_user_attributes() {
        let event: CognitoEventUserPoolsPreTokenGen = serde_json::from_value(cognito_event(
            "TokenGeneration_Authentication",
            json!({
                "userAttributes": cognito_user_attributes(),
                "groupConfiguration": cognito_group_configuration(),
                "clientMetadata": {
                    "source": "test"
                }
            }),
            json!({}),
        ))
        .expect("Cognito Pre token generation event should deserialize");

        let parsed = EventParser::new()
            .parse_cognito_pre_token_generation_user_attributes::<CognitoUserAttributes>(event)
            .expect("Cognito user attributes should parse");

        assert_eq!(parsed.payload().name, "Test User");
    }

    #[test]
    fn parses_cognito_pre_token_generation_v2_user_attributes() {
        let event: CognitoEventUserPoolsPreTokenGenV2 = serde_json::from_value(cognito_event(
            "TokenGeneration_Authentication",
            json!({
                "userAttributes": cognito_user_attributes(),
                "groupConfiguration": cognito_group_configuration(),
                "scopes": ["openid", "email"],
                "clientMetadata": {
                    "source": "test"
                }
            }),
            json!({}),
        ))
        .expect("Cognito Pre token generation v2 event should deserialize");

        let parsed = EventParser::new()
            .parse_cognito_pre_token_generation_v2_user_attributes::<CognitoUserAttributes>(event)
            .expect("Cognito user attributes should parse");

        assert_eq!(parsed.payload().email, "user@example.com");
    }

    #[test]
    fn parses_cognito_post_authentication_user_attributes() {
        let event: CognitoEventUserPoolsPostAuthentication = serde_json::from_value(cognito_event(
            "PostAuthentication_Authentication",
            json!({
                "userAttributes": cognito_user_attributes(),
                "newDeviceUsed": true,
                "clientMetadata": {
                    "source": "test"
                }
            }),
            json!({}),
        ))
        .expect("Cognito Post authentication event should deserialize");

        let parsed = EventParser::new()
            .parse_cognito_post_authentication_user_attributes::<CognitoUserAttributes>(event)
            .expect("Cognito user attributes should parse");

        assert_eq!(parsed.payload().name, "Test User");
    }

    #[test]
    fn parses_cognito_define_auth_challenge_user_attributes() {
        let event: CognitoEventUserPoolsDefineAuthChallenge =
            serde_json::from_value(cognito_event(
                "DefineAuthChallenge_Authentication",
                json!({
                    "userAttributes": cognito_user_attributes(),
                    "session": [],
                    "clientMetadata": {
                        "source": "test"
                    },
                    "userNotFound": false
                }),
                json!({}),
            ))
            .expect("Cognito Define auth challenge event should deserialize");

        let parsed = EventParser::new()
            .parse_cognito_define_auth_challenge_user_attributes::<CognitoUserAttributes>(event)
            .expect("Cognito user attributes should parse");

        assert_eq!(parsed.payload().email, "user@example.com");
    }

    #[test]
    fn parses_cognito_create_auth_challenge_user_attributes() {
        let event: CognitoEventUserPoolsCreateAuthChallenge =
            serde_json::from_value(cognito_event(
                "CreateAuthChallenge_Authentication",
                json!({
                    "userAttributes": cognito_user_attributes(),
                    "challengeName": "CUSTOM_CHALLENGE",
                    "session": [],
                    "clientMetadata": {
                        "source": "test"
                    },
                    "userNotFound": false
                }),
                json!({}),
            ))
            .expect("Cognito Create auth challenge event should deserialize");

        let parsed = EventParser::new()
            .parse_cognito_create_auth_challenge_user_attributes::<CognitoUserAttributes>(event)
            .expect("Cognito user attributes should parse");

        assert_eq!(parsed.payload().name, "Test User");
    }

    #[test]
    fn parses_cognito_verify_auth_challenge_user_attributes() {
        let event: CognitoEventUserPoolsVerifyAuthChallenge =
            serde_json::from_value(cognito_event(
                "VerifyAuthChallengeResponse_Authentication",
                json!({
                    "userAttributes": cognito_user_attributes(),
                    "privateChallengeParameters": {
                        "answer": "expected"
                    },
                    "challengeAnswer": "actual",
                    "clientMetadata": {
                        "source": "test"
                    },
                    "userNotFound": false
                }),
                json!({}),
            ))
            .expect("Cognito Verify auth challenge event should deserialize");

        let parsed = EventParser::new()
            .parse_cognito_verify_auth_challenge_user_attributes::<CognitoUserAttributes>(event)
            .expect("Cognito user attributes should parse");

        assert_eq!(parsed.payload().email, "user@example.com");
    }

    #[test]
    fn parses_cognito_custom_message_user_attributes() {
        let event: CognitoEventUserPoolsCustomMessage = serde_json::from_value(cognito_event(
            "CustomMessage_SignUp",
            json!({
                "userAttributes": cognito_user_attributes(),
                "codeParameter": "{####}",
                "usernameParameter": "test-user",
                "clientMetadata": {
                    "source": "test"
                }
            }),
            json!({}),
        ))
        .expect("Cognito Custom message event should deserialize");

        let parsed = EventParser::new()
            .parse_cognito_custom_message_user_attributes::<CognitoUserAttributes>(event)
            .expect("Cognito user attributes should parse");

        assert_eq!(parsed.payload().name, "Test User");
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
    fn parses_kinesis_cloudwatch_log_messages() {
        let event: KinesisEvent = serde_json::from_value(json!({
            "Records": [
                {
                    "kinesis": {
                        "kinesisSchemaVersion": "1.0",
                        "partitionKey": "logs",
                        "sequenceNumber": "1",
                        "data": "H4sIACNFBWoAA4WOPQ+CMBCG/8vNZWjxk41EZHKCzRKC0pAm0GJbNITw3z1EBlzc7p6797kboBHWFpVI+1ZAAKcwDfNLlCRhHAEB/VLCIKbM32x3+8MRC8S1rmKjuxYn2pTC2JklzoiiQWjngoDtbvZuZOukVmdZu2k1uC6h7JOKnkK5CQ8gSwyLqfcopp3E51zR4B1KlkdxY+CzIZclh+DbeJQD4fDoCuWk63HARhjJSsrWUvZfyn6lPkqz8Q3Yvl/uNwEAAA==",
                        "approximateArrivalTimestamp": 1
                    }
                }
            ]
        }))
        .expect("Kinesis event should deserialize");

        let parsed = EventParser::new()
            .parse_kinesis_cloudwatch_log_messages::<OrderEvent>(event)
            .expect("Kinesis CloudWatch Logs messages should parse");

        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].payload().order_id, "order-1");
        assert_eq!(parsed[1].payload().quantity, 3);
    }

    #[test]
    fn rejects_invalid_kinesis_cloudwatch_log_payloads() {
        let event: KinesisEvent = serde_json::from_value(json!({
            "Records": [
                {
                    "kinesis": {
                        "kinesisSchemaVersion": "1.0",
                        "partitionKey": "logs",
                        "sequenceNumber": "1",
                        "data": "e30=",
                        "approximateArrivalTimestamp": 1
                    }
                }
            ]
        }))
        .expect("Kinesis event should deserialize");

        let error = EventParser::new()
            .parse_kinesis_cloudwatch_log_messages::<OrderEvent>(event)
            .expect_err("invalid CloudWatch Logs payload should fail");

        assert_eq!(error.kind(), ParseErrorKind::Data);
        assert!(error.message().contains("CloudWatch Logs payload"));
    }

    #[test]
    fn parses_kinesis_dynamodb_new_images() {
        let dynamodb_record = json!({
            "awsRegion": "us-east-1",
            "eventID": "event-1",
            "eventName": "INSERT",
            "eventSource": "aws:dynamodb",
            "recordFormat": "application/json",
            "tableName": "orders",
            "dynamodb": {
                "Keys": {
                    "order_id": {
                        "S": "order-1"
                    }
                },
                "NewImage": {
                    "order_id": {
                        "S": "order-1"
                    },
                    "quantity": {
                        "N": "2"
                    }
                },
                "SizeBytes": 42
            }
        });
        let event: KinesisEvent = serde_json::from_value(json!({
            "Records": [
                {
                    "kinesis": {
                        "kinesisSchemaVersion": "1.0",
                        "partitionKey": "orders",
                        "sequenceNumber": "1",
                        "data": STANDARD.encode(dynamodb_record.to_string()),
                        "approximateArrivalTimestamp": 1
                    }
                }
            ]
        }))
        .expect("Kinesis event should deserialize");

        let parsed = EventParser::new()
            .parse_kinesis_dynamodb_new_images::<OrderEvent>(event)
            .expect("Kinesis DynamoDB NewImage should parse");

        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].payload().order_id, "order-1");
        assert_eq!(parsed[0].payload().quantity, 2);
    }

    #[test]
    fn parses_kinesis_dynamodb_old_images() {
        let dynamodb_record = json!({
            "awsRegion": "us-east-1",
            "eventID": "event-1",
            "eventName": "MODIFY",
            "eventSource": "aws:dynamodb",
            "recordFormat": "application/json",
            "tableName": "orders",
            "dynamodb": {
                "Keys": {
                    "order_id": {
                        "S": "order-1"
                    }
                },
                "OldImage": {
                    "order_id": {
                        "S": "order-1"
                    },
                    "quantity": {
                        "N": "1"
                    }
                },
                "SizeBytes": 42
            }
        });
        let event: KinesisEvent = serde_json::from_value(json!({
            "Records": [
                {
                    "kinesis": {
                        "kinesisSchemaVersion": "1.0",
                        "partitionKey": "orders",
                        "sequenceNumber": "1",
                        "data": STANDARD.encode(dynamodb_record.to_string()),
                        "approximateArrivalTimestamp": 1
                    }
                }
            ]
        }))
        .expect("Kinesis event should deserialize");

        let parsed = EventParser::new()
            .parse_kinesis_dynamodb_old_images::<OrderEvent>(event)
            .expect("Kinesis DynamoDB OldImage should parse");

        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].payload().order_id, "order-1");
        assert_eq!(parsed[0].payload().quantity, 1);
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
    fn parses_firehose_sqs_message_bodies() {
        let sqs_message = json!({
            "messageId": "message-1",
            "body": r#"{"order_id":"order-1","quantity":2}"#,
            "eventSource": "aws:sqs",
            "awsRegion": "us-east-1"
        });
        let event: KinesisFirehoseEvent = serde_json::from_value(json!({
            "records": [
                {
                    "recordId": "record-1",
                    "approximateArrivalTimestamp": 1,
                    "data": STANDARD.encode(sqs_message.to_string())
                }
            ]
        }))
        .expect("firehose event should deserialize");

        let parsed = EventParser::new()
            .parse_firehose_sqs_message_bodies::<OrderEvent>(event)
            .expect("valid Firehose SQS records should parse");

        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].payload().order_id, "order-1");
        assert_eq!(parsed[0].payload().quantity, 2);
    }

    #[test]
    fn rejects_firehose_sqs_record_without_sqs_message() {
        let event: KinesisFirehoseEvent = serde_json::from_value(json!({
            "records": [
                {
                    "recordId": "record-1",
                    "approximateArrivalTimestamp": 1,
                    "data": STANDARD.encode(r#"{"not":"an SQS message"}"#)
                }
            ]
        }))
        .expect("firehose event should deserialize");

        let error = EventParser::new()
            .parse_firehose_sqs_message_bodies::<OrderEvent>(event)
            .expect_err("missing SQS body should fail");

        assert_eq!(error.kind(), ParseErrorKind::Data);
        assert!(error.message().contains("missing body"));
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
    fn parses_dynamodb_new_and_old_images() {
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
                },
                {
                    "eventID": "2",
                    "eventName": "MODIFY",
                    "awsRegion": "us-east-1",
                    "eventSource": "aws:dynamodb",
                    "dynamodb": {
                        "ApproximateCreationDateTime": 2,
                        "Keys": {
                            "order_id": {"S": "order-2"}
                        },
                        "OldImage": {
                            "order_id": {"S": "order-2"},
                            "quantity": {"N": "1"}
                        },
                        "NewImage": {
                            "order_id": {"S": "order-2"},
                            "quantity": {"N": "3"}
                        },
                        "SequenceNumber": "2",
                        "SizeBytes": 52,
                        "StreamViewType": "NEW_AND_OLD_IMAGES"
                    }
                }
            ]
        }))
        .expect("DynamoDB event should deserialize");

        let parsed = EventParser::new()
            .parse_dynamodb_images::<OrderEvent>(event)
            .expect("new and old images should parse");

        assert_eq!(parsed.len(), 2);
        assert_eq!(
            parsed[0]
                .new_image()
                .expect("first record should have NewImage")
                .payload()
                .quantity,
            2
        );
        assert!(parsed[0].old_image().is_none());
        assert_eq!(
            parsed[1]
                .old_image()
                .expect("second record should have OldImage")
                .payload()
                .quantity,
            1
        );
        assert_eq!(
            parsed[1]
                .new_image()
                .expect("second record should have NewImage")
                .payload()
                .quantity,
            3
        );
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
    fn parses_s3_records() {
        let event: S3Event = serde_json::from_value(json!({
            "Records": [
                {
                    "eventVersion": "2.1",
                    "eventSource": "aws:s3",
                    "awsRegion": "us-east-1",
                    "eventTime": "2023-04-12T20:43:38.021Z",
                    "eventName": "ObjectCreated:Put",
                    "userIdentity": {
                        "principalId": "A1YQ72UWCM96UF"
                    },
                    "requestParameters": {
                        "sourceIPAddress": "93.108.161.96"
                    },
                    "responseElements": {
                        "x-amz-request-id": "request-1"
                    },
                    "s3": {
                        "s3SchemaVersion": "1.0",
                        "configurationId": "config-1",
                        "bucket": {
                            "name": "orders",
                            "ownerIdentity": {
                                "principalId": "A1YQ72UWCM96UF"
                            },
                            "arn": "arn:aws:s3:::orders"
                        },
                        "object": {
                            "key": "order-1.json",
                            "size": 512,
                            "eTag": "etag-1",
                            "sequencer": "001"
                        }
                    }
                }
            ]
        }))
        .expect("S3 event should deserialize");

        let parsed = EventParser::new()
            .parse_s3_records::<S3RecordSummary>(event)
            .expect("S3 records should parse");

        assert_eq!(
            parsed[0].payload().event_name.as_deref(),
            Some("ObjectCreated:Put")
        );
        assert_eq!(
            parsed[0].payload().s3.bucket.name.as_deref(),
            Some("orders")
        );
        assert_eq!(
            parsed[0].payload().s3.object.key.as_deref(),
            Some("order-1.json")
        );
        assert_eq!(parsed[0].payload().s3.object.size, Some(512));
    }

    #[test]
    fn parses_s3_event_notification_records() {
        let event = serde_json::from_value(json!({
            "Records": [
                {
                    "eventVersion": "2.3",
                    "eventSource": "aws:s3",
                    "awsRegion": "us-east-1",
                    "eventTime": "2025-09-29T00:47:23.967Z",
                    "eventName": "IntelligentTiering",
                    "userIdentity": {
                        "principalId": "s3.amazonaws.com"
                    },
                    "requestParameters": {
                        "sourceIPAddress": "s3.amazonaws.com"
                    },
                    "responseElements": {
                        "x-amz-request-id": "request-1",
                        "x-amz-id-2": "host-1"
                    },
                    "s3": {
                        "s3SchemaVersion": "1.0",
                        "configurationId": "config-1",
                        "bucket": {
                            "name": "orders",
                            "ownerIdentity": {
                                "principalId": "owner-1"
                            },
                            "arn": "arn:aws:s3:::orders"
                        },
                        "get_object": {
                            "key": "archive/order-1.json",
                            "size": 252_294,
                            "eTag": "etag-1",
                            "versionId": "version-1",
                            "sequencer": "001"
                        }
                    },
                    "intelligentTieringEventData": {
                        "destinationAccessTier": "DEEP_ARCHIVE_ACCESS"
                    }
                }
            ]
        }))
        .expect("S3 event notification should deserialize");

        let parsed = EventParser::new()
            .parse_s3_event_notification_records::<S3NotificationRecordSummary>(event)
            .expect("S3 event notification records should parse");

        assert_eq!(parsed[0].payload().event_name, "IntelligentTiering");
        assert_eq!(
            parsed[0].payload().s3.bucket.name.as_deref(),
            Some("orders")
        );
        assert_eq!(
            parsed[0]
                .payload()
                .s3
                .get_object
                .as_ref()
                .and_then(|object| object.key.as_deref()),
            Some("archive/order-1.json")
        );
        assert_eq!(
            parsed[0]
                .payload()
                .intelligent_tiering_event_data
                .as_ref()
                .map(|data| data.destination_access_tier.as_str()),
            Some("DEEP_ARCHIVE_ACCESS")
        );
    }

    #[test]
    fn parses_s3_object_lambda_configuration_payload() {
        let event: S3ObjectLambdaEvent<Value> = serde_json::from_value(json!({
            "xAmzRequestId": "request-1",
            "getObjectContext": {
                "inputS3Url": "https://s3.amazonaws.com/orders/order-1.json",
                "outputRoute": "route",
                "outputToken": "token"
            },
            "configuration": {
                "accessPointArn": "arn:aws:s3-object-lambda:us-east-1:123456789012:accesspoint/orders",
                "supportingAccessPointArn": "arn:aws:s3:us-east-1:123456789012:accesspoint/orders-support",
                "payload": {
                    "tenant": "tenant-1",
                    "redact": true
                }
            },
            "userRequest": {
                "url": "/orders/order-1.json",
                "headers": {
                    "Accept": "application/json"
                }
            },
            "userIdentity": {
                "type": "IAMUser",
                "principalId": "principal-1",
                "arn": "arn:aws:iam::123456789012:user/test",
                "accountId": "123456789012",
                "accessKeyId": "access-key-1"
            },
            "protocolVersion": "1.00"
        }))
        .expect("S3 Object Lambda event should deserialize");

        let parsed = EventParser::new()
            .parse_s3_object_lambda_configuration_payload::<S3ObjectLambdaPayload>(event)
            .expect("S3 Object Lambda payload should parse");

        assert_eq!(parsed.payload().tenant, "tenant-1");
        assert!(parsed.payload().redact);
    }

    #[test]
    fn parses_s3_batch_job_tasks() {
        let event: S3BatchJobEvent = serde_json::from_value(json!({
            "invocationSchemaVersion": "1.0",
            "invocationId": "invocation-1",
            "job": {
                "id": "job-1"
            },
            "tasks": [
                {
                    "taskId": "task-1",
                    "s3Key": "orders/order-1.json",
                    "s3BucketArn": "arn:aws:s3:::orders"
                },
                {
                    "taskId": "task-2",
                    "s3Key": "orders/order-2.json",
                    "s3BucketArn": "arn:aws:s3:::orders"
                }
            ]
        }))
        .expect("S3 Batch job event should deserialize");

        let parsed = EventParser::new()
            .parse_s3_batch_job_tasks::<S3BatchTaskSummary>(event)
            .expect("S3 Batch tasks should parse");

        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].payload().task_id.as_deref(), Some("task-1"));
        assert_eq!(
            parsed[1].payload().s3_key.as_deref(),
            Some("orders/order-2.json")
        );
        assert_eq!(
            parsed[1].payload().s3_bucket_arn.as_deref(),
            Some("arn:aws:s3:::orders")
        );
    }

    #[test]
    fn parses_s3_sqs_event_records() {
        let s3_body = json!({
            "Records": [
                {
                    "eventVersion": "2.1",
                    "eventSource": "aws:s3",
                    "awsRegion": "us-east-1",
                    "eventTime": "2023-04-12T20:43:38.021Z",
                    "eventName": "ObjectCreated:Put",
                    "userIdentity": {
                        "principalId": "A1YQ72UWCM96UF"
                    },
                    "requestParameters": {
                        "sourceIPAddress": "93.108.161.96"
                    },
                    "responseElements": {
                        "x-amz-request-id": "request-1"
                    },
                    "s3": {
                        "s3SchemaVersion": "1.0",
                        "configurationId": "config-1",
                        "bucket": {
                            "name": "orders",
                            "ownerIdentity": {
                                "principalId": "A1YQ72UWCM96UF"
                            },
                            "arn": "arn:aws:s3:::orders"
                        },
                        "object": {
                            "key": "order-1.json",
                            "size": 512,
                            "eTag": "etag-1",
                            "sequencer": "001"
                        }
                    }
                }
            ]
        });
        let mut message = SqsMessage::default();
        message.body = Some(s3_body.to_string());
        let mut event = SqsEvent::default();
        event.records = vec![message];

        let parsed = EventParser::new()
            .parse_s3_sqs_event_records::<S3RecordSummary>(event)
            .expect("S3 notifications should parse");

        assert_eq!(parsed.len(), 1);
        assert_eq!(
            parsed[0].payload().s3.object.key.as_deref(),
            Some("order-1.json")
        );
    }

    #[test]
    fn parses_s3_sqs_event_notification_records() {
        let s3_body = json!({
            "Records": [
                {
                    "eventVersion": "2.3",
                    "eventSource": "aws:s3",
                    "awsRegion": "us-east-1",
                    "eventTime": "2025-09-29T00:47:23.967Z",
                    "eventName": "IntelligentTiering",
                    "userIdentity": {
                        "principalId": "s3.amazonaws.com"
                    },
                    "requestParameters": {
                        "sourceIPAddress": "s3.amazonaws.com"
                    },
                    "responseElements": {
                        "x-amz-request-id": "request-1",
                        "x-amz-id-2": "host-1"
                    },
                    "s3": {
                        "s3SchemaVersion": "1.0",
                        "configurationId": "config-1",
                        "bucket": {
                            "name": "orders",
                            "ownerIdentity": {
                                "principalId": "owner-1"
                            },
                            "arn": "arn:aws:s3:::orders"
                        },
                        "get_object": {
                            "key": "archive/order-1.json",
                            "size": 252_294,
                            "eTag": "etag-1"
                        }
                    },
                    "intelligentTieringEventData": {
                        "destinationAccessTier": "ARCHIVE_ACCESS"
                    }
                }
            ]
        });
        let mut message = SqsMessage::default();
        message.body = Some(s3_body.to_string());
        let mut event = SqsEvent::default();
        event.records = vec![message];

        let parsed = EventParser::new()
            .parse_s3_sqs_event_notification_records::<S3NotificationRecordSummary>(event)
            .expect("S3 event notifications should parse");

        assert_eq!(parsed.len(), 1);
        assert_eq!(
            parsed[0]
                .payload()
                .s3
                .get_object
                .as_ref()
                .and_then(|object| object.key.as_deref()),
            Some("archive/order-1.json")
        );
    }

    #[test]
    fn rejects_s3_sqs_records_without_s3_notifications() {
        let mut message = SqsMessage::default();
        message.body = Some(r#"{"not":"an S3 event"}"#.to_owned());
        let mut event = SqsEvent::default();
        event.records = vec![message];

        let error = EventParser::new()
            .parse_s3_sqs_event_records::<S3RecordSummary>(event)
            .expect_err("invalid S3 notification should fail");

        assert_eq!(error.kind(), ParseErrorKind::Data);
        assert!(error.message().contains("S3 event notification"));
    }

    #[test]
    fn parses_ses_records() {
        let event: SimpleEmailEvent = serde_json::from_value(json!({
            "Records": [
                {
                    "eventVersion": "1.0",
                    "eventSource": "aws:ses",
                    "ses": {
                        "mail": {
                            "timestamp": "2023-04-12T20:43:38.021Z",
                            "source": "sender@example.com",
                            "messageId": "message-1",
                            "destination": ["recipient@example.com"],
                            "headersTruncated": false,
                            "headers": [
                                {
                                    "name": "Subject",
                                    "value": "Order received"
                                }
                            ],
                            "commonHeaders": {
                                "from": ["sender@example.com"],
                                "to": ["recipient@example.com"],
                                "messageId": "message-1",
                                "subject": "Order received"
                            }
                        },
                        "receipt": {
                            "timestamp": "2023-04-12T20:43:38.021Z",
                            "processingTimeMillis": 100,
                            "recipients": ["recipient@example.com"],
                            "spamVerdict": {
                                "status": "PASS"
                            },
                            "virusVerdict": {
                                "status": "PASS"
                            },
                            "spfVerdict": {
                                "status": "PASS"
                            },
                            "dkimVerdict": {
                                "status": "PASS"
                            },
                            "dmarcVerdict": {
                                "status": "PASS"
                            },
                            "action": {
                                "type": "Lambda",
                                "functionArn": "arn:aws:lambda:us-east-1:123456789012:function:handler",
                                "invocationType": "Event"
                            }
                        }
                    }
                }
            ]
        }))
        .expect("SES event should deserialize");

        let parsed = EventParser::new()
            .parse_ses_records::<SesRecordSummary>(event)
            .expect("SES records should parse");

        assert_eq!(parsed[0].payload().event_source.as_deref(), Some("aws:ses"));
        assert_eq!(
            parsed[0].payload().ses.mail.message_id.as_deref(),
            Some("message-1")
        );
        assert_eq!(
            parsed[0]
                .payload()
                .ses
                .mail
                .common_headers
                .subject
                .as_deref(),
            Some("Order received")
        );
        assert_eq!(
            parsed[0].payload().ses.receipt.action.type_.as_deref(),
            Some("Lambda")
        );
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
    fn parses_sns_sqs_messages() {
        let notification = json!({
            "Type": "Notification",
            "MessageId": "message-1",
            "TopicArn": "arn:aws:sns:us-east-1:123456789012:orders",
            "Subject": "Order",
            "Message": "{\"order_id\":\"order-1\",\"quantity\":2}",
            "Timestamp": "2019-01-02T12:45:07.000Z",
            "SignatureVersion": "1",
            "Signature": "signature",
            "SigningCertURL": "https://sns.us-east-1.amazonaws.com/cert.pem",
            "UnsubscribeURL": "https://sns.us-east-1.amazonaws.com/unsubscribe",
            "MessageAttributes": {}
        });
        let mut message = SqsMessage::default();
        message.body = Some(notification.to_string());
        let mut event = SqsEvent::default();
        event.records = vec![message];

        let parsed = EventParser::new()
            .parse_sns_sqs_messages::<OrderEvent>(event)
            .expect("SNS notification should parse");

        assert_eq!(parsed[0].payload().order_id, "order-1");
        assert_eq!(parsed[0].payload().quantity, 2);
    }

    #[test]
    fn rejects_sns_sqs_records_without_sns_notifications() {
        let mut message = SqsMessage::default();
        message.body =
            Some(r#"{"Message":"{\"order_id\":\"order-1\",\"quantity\":2}"}"#.to_owned());
        let mut event = SqsEvent::default();
        event.records = vec![message];

        let error = EventParser::new()
            .parse_sns_sqs_messages::<OrderEvent>(event)
            .expect_err("invalid SNS notification should fail");

        assert_eq!(error.kind(), ParseErrorKind::Data);
        assert!(error.message().contains("SNS notification"));
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
