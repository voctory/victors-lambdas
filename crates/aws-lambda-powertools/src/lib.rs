//! Umbrella crate for Powertools Lambda Rust utilities.

/// Batch processing utility.
#[cfg(feature = "batch")]
pub mod batch {
    pub use aws_lambda_powertools_batch::*;
}

/// Shared configuration and runtime helpers.
pub mod core {
    pub use aws_lambda_powertools_core::*;
}

/// Data masking utility.
#[cfg(feature = "data-masking")]
pub mod data_masking {
    pub use aws_lambda_powertools_data_masking::*;
}

/// Event handler utility.
#[cfg(feature = "event-handler")]
pub mod event_handler {
    pub use aws_lambda_powertools_event_handler::*;
}

/// Feature flag evaluation utility.
#[cfg(feature = "feature-flags")]
pub mod feature_flags {
    pub use aws_lambda_powertools_feature_flags::*;
}

/// Idempotency utility.
#[cfg(feature = "idempotency")]
pub mod idempotency {
    pub use aws_lambda_powertools_idempotency::*;
}

/// `JMESPath` extraction utility.
#[cfg(feature = "jmespath")]
pub mod jmespath {
    pub use aws_lambda_powertools_jmespath::*;
}

/// Kafka consumer record utility.
#[cfg(feature = "kafka")]
pub mod kafka {
    pub use aws_lambda_powertools_kafka::*;
}

/// Structured logging utility.
#[cfg(feature = "logger")]
pub mod logger {
    pub use aws_lambda_powertools_logger::*;
}

/// Metrics utility.
#[cfg(feature = "metrics")]
pub mod metrics {
    pub use aws_lambda_powertools_metrics::*;
}

/// Parameter retrieval utility.
#[cfg(feature = "parameters")]
pub mod parameters {
    pub use aws_lambda_powertools_parameters::*;
}

/// Event parsing utility.
#[cfg(feature = "parser")]
pub mod parser {
    pub use aws_lambda_powertools_parser::*;
}

/// Seekable streaming utility.
#[cfg(feature = "streaming")]
pub mod streaming {
    pub use aws_lambda_powertools_streaming::*;
}

/// Common imports for Lambda handlers.
pub mod prelude {
    pub use aws_lambda_powertools_core::{ServiceConfig, ServiceConfigBuilder};

    #[cfg(feature = "batch")]
    pub use aws_lambda_powertools_batch::{
        BatchItemFailure, BatchProcessingReport, BatchProcessor, BatchRecord, BatchRecordResult,
        BatchResponse,
    };

    #[cfg(feature = "batch-parser")]
    pub use aws_lambda_powertools_batch::ParsedBatchRecord;

    #[cfg(feature = "data-masking")]
    pub use aws_lambda_powertools_data_masking::{
        DATA_MASKING_STRING, DataMasking, DataMaskingConfig, DataMaskingError,
        DataMaskingErrorKind, DataMaskingProvider, DataMaskingResult, EncryptionContext,
        MaskingOptions, MaskingStrategy, erase, erase_fields,
    };

    #[cfg(feature = "data-masking-kms")]
    pub use aws_lambda_powertools_data_masking::KmsDataMaskingProvider;

    #[cfg(feature = "event-handler")]
    pub use aws_lambda_powertools_event_handler::{
        AsyncErrorHandler, AsyncFallibleHandler, AsyncHandler, AsyncRoute, AsyncRouteMatch,
        AsyncRouter, CorsConfig, ErrorHandler, Extensions, FallibleHandler, FallibleResponseFuture,
        Handler, HttpError, Method, ParseMethodError, PathParams, Request, RequestMiddleware,
        Response, ResponseFuture, ResponseMiddleware, Route, RouteError, RouteMatch, RouteResult,
        Router,
    };

    #[cfg(feature = "event-handler-validation")]
    pub use aws_lambda_powertools_event_handler::{
        RequestValidator, ResponseValidator, ValidationConfig as EventHandlerValidationConfig,
    };

    #[cfg(feature = "event-handler-aws-lambda-events")]
    pub use aws_lambda_powertools_event_handler::{
        AlbAdapterError, AlbAdapterResult, ApiGatewayAdapterError, ApiGatewayAdapterResult,
        AppSyncBatchHandler, AppSyncBatchResponseFuture, AppSyncBatchRoute, AppSyncEvent,
        AppSyncHandler, AppSyncResolver, AppSyncResolverError, AppSyncResolverResult,
        AppSyncResponseFuture, AppSyncRoute, AsyncAppSyncBatchHandler, AsyncAppSyncBatchRoute,
        AsyncAppSyncHandler, AsyncAppSyncResolver, AsyncAppSyncRoute, BedrockAgentAdapterError,
        BedrockAgentAdapterResult, LambdaFunctionUrlAdapterError, LambdaFunctionUrlAdapterResult,
        VpcLatticeAdapterError, VpcLatticeAdapterResult, request_from_alb, request_from_apigw_v1,
        request_from_apigw_v2, request_from_apigw_websocket, request_from_bedrock_agent,
        request_from_lambda_function_url, request_from_vpc_lattice, request_from_vpc_lattice_v2,
        response_to_alb, response_to_apigw_v1, response_to_apigw_v2, response_to_apigw_websocket,
        response_to_bedrock_agent, response_to_lambda_function_url, response_to_vpc_lattice,
    };

    #[cfg(feature = "event-handler-appsync-events")]
    pub use aws_lambda_powertools_event_handler::{
        AppSyncEventsAggregatePublishHandler, AppSyncEventsHandlerError,
        AppSyncEventsHandlerResult, AppSyncEventsPublishHandler, AppSyncEventsPublishRoute,
        AppSyncEventsResolver, AppSyncEventsResolverError, AppSyncEventsResolverResult,
        AppSyncEventsSubscribeHandler, AppSyncEventsSubscribeRoute,
    };

    #[cfg(feature = "event-handler-bedrock-agent-functions")]
    pub use aws_lambda_powertools_event_handler::{
        AsyncBedrockAgentFunctionHandler, AsyncBedrockAgentFunctionResolver,
        AsyncBedrockFunctionRoute, BedrockAgentFunctionAgent, BedrockAgentFunctionEvent,
        BedrockAgentFunctionHandler, BedrockAgentFunctionHandlerError,
        BedrockAgentFunctionHandlerResult, BedrockAgentFunctionParameter,
        BedrockAgentFunctionParameterValue, BedrockAgentFunctionParameters,
        BedrockAgentFunctionResolver, BedrockAgentFunctionResponseFuture,
        BedrockAgentFunctionResponseState, BedrockFunctionResponse, BedrockFunctionResult,
        BedrockFunctionRoute,
    };

    #[cfg(feature = "feature-flags")]
    pub use aws_lambda_powertools_feature_flags::{
        AsyncFeatureFlagStore, AsyncFeatureFlags, FeatureCondition, FeatureFlag,
        FeatureFlagCachePolicy, FeatureFlagConfig, FeatureFlagContext, FeatureFlagError,
        FeatureFlagErrorKind, FeatureFlagFuture, FeatureFlagResult, FeatureFlagStore, FeatureFlags,
        FeatureRule, InMemoryFeatureFlagStore, RuleAction,
    };

    #[cfg(feature = "feature-flags-appconfig")]
    pub use aws_lambda_powertools_feature_flags::AppConfigFeatureFlagStore;

    #[cfg(feature = "idempotency")]
    pub use aws_lambda_powertools_idempotency::{
        AsyncIdempotency, AsyncIdempotencyStore, CachedIdempotencyStore, Idempotency,
        IdempotencyConfig, IdempotencyError, IdempotencyExecutionError, IdempotencyKey,
        IdempotencyOutcome, IdempotencyRecord, IdempotencyResult, IdempotencyStatus,
        IdempotencyStore, IdempotencyStoreError, IdempotencyStoreFuture, IdempotencyStoreResult,
        InMemoryIdempotencyStore, hash_payload, key_from_json_pointer, key_from_payload,
    };

    #[cfg(feature = "idempotency-dynamodb")]
    pub use aws_lambda_powertools_idempotency::DynamoDbIdempotencyStore;

    #[cfg(feature = "jmespath")]
    pub use aws_lambda_powertools_jmespath::{
        API_GATEWAY_HTTP, API_GATEWAY_REST, CLOUDWATCH_EVENTS_SCHEDULED, CLOUDWATCH_LOGS,
        EVENTBRIDGE, JmespathError, JmespathErrorKind, JmespathExpression, JmespathResult,
        KINESIS_DATA_STREAM, S3_EVENTBRIDGE_SQS, S3_KINESIS_FIREHOSE, S3_SNS_KINESIS_FIREHOSE,
        S3_SNS_SQS, S3_SQS, SNS, SQS, extract_data_from_envelope, query, search, search_as,
    };

    #[cfg(feature = "kafka")]
    pub use aws_lambda_powertools_kafka::{
        ConsumerRecord, ConsumerRecords, KafkaConsumer, KafkaConsumerConfig, KafkaConsumerError,
        KafkaConsumerErrorKind, KafkaConsumerResult, KafkaFieldDeserializer, consumer_records,
        decode_base64_json, decode_base64_string,
    };

    #[cfg(feature = "kafka-avro")]
    pub use aws_lambda_powertools_kafka::decode_base64_avro;

    #[cfg(feature = "kafka-protobuf")]
    pub use aws_lambda_powertools_kafka::{
        ProtobufWireFormat, decode_base64_protobuf, decode_base64_protobuf_with_format,
    };

    #[cfg(feature = "logger")]
    pub use aws_lambda_powertools_logger::{
        JsonLogFormatter, LambdaContextFields, LambdaLogContext, LogEntry, LogFields, LogFormatter,
        LogLevel, LogRedactor, LogValue, Logger, LoggerConfig,
    };

    #[cfg(feature = "logger-tracing")]
    pub use aws_lambda_powertools_logger::LoggerLayer;

    #[cfg(feature = "metrics")]
    pub use aws_lambda_powertools_metrics::{
        MetadataValue, Metric, MetricResolution, MetricUnit, Metrics, MetricsConfig, MetricsError,
        MetricsFuture,
    };

    #[cfg(feature = "parameters")]
    pub use aws_lambda_powertools_parameters::{
        AsyncParameterError, AsyncParameterProvider, AsyncParameterResult, AsyncParameters,
        CachePolicy, InMemoryParameterProvider, Parameter, ParameterFuture, ParameterProvider,
        ParameterProviderError, ParameterProviderResult, ParameterTransform,
        ParameterTransformError, ParameterTransformErrorKind, ParameterValue, Parameters,
    };

    #[cfg(feature = "parameters-appconfig")]
    pub use aws_lambda_powertools_parameters::AppConfigProvider;

    #[cfg(feature = "parameters-dynamodb")]
    pub use aws_lambda_powertools_parameters::DynamoDbParameterProvider;

    #[cfg(feature = "parameters-secrets")]
    pub use aws_lambda_powertools_parameters::SecretsManagerProvider;

    #[cfg(feature = "parameters-ssm")]
    pub use aws_lambda_powertools_parameters::{
        SsmParameterProvider, SsmParameterType, SsmParametersByName,
    };

    #[cfg(feature = "parser")]
    pub use aws_lambda_powertools_parser::{
        AppSyncEventsChannel, AppSyncEventsChannelNamespace, AppSyncEventsCognitoIdentity,
        AppSyncEventsEvent, AppSyncEventsIamIdentity, AppSyncEventsIdentity,
        AppSyncEventsIncomingEvent, AppSyncEventsInfo, AppSyncEventsLambdaIdentity,
        AppSyncEventsModel, AppSyncEventsOidcIdentity, AppSyncEventsOperation,
        AppSyncEventsRequest, BedrockAgentEvent, BedrockAgentEventModel,
        BedrockAgentFunctionEventModel, BedrockAgentModel, BedrockAgentPropertyModel,
        BedrockAgentRequestBody, BedrockAgentRequestBodyModel, BedrockAgentRequestMedia,
        BedrockAgentRequestMediaModel, CognitoCustomEmailSenderTriggerEvent,
        CognitoCustomEmailSenderTriggerModel, CognitoCustomEmailSenderTriggerSource,
        CognitoCustomSMSSenderTriggerModel, CognitoCustomSenderRequest,
        CognitoCustomSenderRequestType, CognitoCustomSmsSenderTriggerEvent,
        CognitoCustomSmsSenderTriggerSource, CognitoMigrateUserRequest, CognitoMigrateUserResponse,
        CognitoMigrateUserTriggerEvent, CognitoMigrateUserTriggerModel,
        CognitoMigrateUserTriggerSource, CognitoUserPoolCallerContext, DynamoDbStreamBatchInfo,
        DynamoDbStreamImageRecord, DynamoDbStreamOnFailureDestination,
        DynamoDbStreamRequestContext, DynamoDbStreamResponseContext, EventParser,
        IoTCoreAddOrDeleteFromThingGroupEvent, IoTCoreAddOrRemoveFromThingGroupEvent,
        IoTCorePropagatingAttribute, IoTCoreRegistryCrudOperation, IoTCoreRegistryEventType,
        IoTCoreRegistryMembershipOperation, IoTCoreThingEvent, IoTCoreThingGroupEvent,
        IoTCoreThingGroupHierarchyEvent, IoTCoreThingGroupMembershipEvent,
        IoTCoreThingGroupReference, IoTCoreThingTypeAssociationEvent, IoTCoreThingTypeEvent,
        ParseError, ParseErrorKind, ParsedEvent, S3EventBridgeBucket, S3EventBridgeDetail,
        S3EventBridgeEvent, S3EventBridgeObject, S3EventNotification, S3EventNotificationBucket,
        S3EventNotificationEntity, S3EventNotificationEventBridgeDetailModel,
        S3EventNotificationEventBridgeModel, S3EventNotificationGlacierEventData,
        S3EventNotificationGlacierRestoreEventData, S3EventNotificationIdentity,
        S3EventNotificationIntelligentTieringEventData, S3EventNotificationModel,
        S3EventNotificationObject, S3EventNotificationRecord, S3EventNotificationRecordModel,
        S3EventNotificationRequestParameters, S3EventNotificationResponseElements,
        S3EventRecordIntelligentTieringEventData, S3RecordModel, TransferFamilyAuthorizerEvent,
        TransferFamilyAuthorizerResponse, TransferFamilyHomeDirectoryEntry,
        TransferFamilyHomeDirectoryType, TransferFamilyPosixProfile, TransferFamilyProtocol,
        TransferFamilyResponseError, TransferFamilyResponseResult,
    };

    #[cfg(feature = "parser-aws-lambda-events")]
    pub use aws_lambda_powertools_parser::{
        ActiveMqModel, AlbModel, ApiGatewayAuthorizerHttpApiV1Request,
        ApiGatewayAuthorizerIamPolicyResponse, ApiGatewayAuthorizerRequest,
        ApiGatewayAuthorizerRequestV2, ApiGatewayAuthorizerResponse,
        ApiGatewayAuthorizerSimpleResponse, ApiGatewayAuthorizerToken, ApiGatewayProxyEventModel,
        ApiGatewayProxyEventV2Model, ApiGatewayWebsocketConnectEvent,
        ApiGatewayWebsocketDisconnectEvent, ApiGatewayWebsocketMessageEvent,
        AppSyncBatchResolverEvent, AppSyncResolverEvent, CloudFormationCustomResourceCreate,
        CloudFormationCustomResourceDelete, CloudFormationCustomResourceRequest,
        CloudFormationCustomResourceResponse, CloudFormationCustomResourceResponseStatus,
        CloudFormationCustomResourceUpdate, CloudWatchLogsModel,
        CognitoCreateAuthChallengeTriggerModel, CognitoCustomMessageTriggerModel,
        CognitoDefineAuthChallengeTriggerModel, CognitoPostAuthenticationTriggerModel,
        CognitoPostConfirmationTriggerModel, CognitoPreAuthenticationTriggerModel,
        CognitoPreSignupTriggerModel, CognitoPreTokenGenerationTriggerModelV1,
        CognitoPreTokenGenerationTriggerModelV2AndV3, CognitoVerifyAuthChallengeTriggerModel,
        DynamoDbStreamModel, EventBridgeModel, KafkaMskEventModel, KafkaSelfManagedEventModel,
        KinesisDataStreamModel, KinesisFirehoseModel, KinesisFirehoseSqsModel,
        LambdaFunctionUrlModel, RabbitMqModel, S3BatchOperationModel, S3Model, S3ObjectLambdaEvent,
        S3SqsEventNotificationModel, SesModel, SnsModel, SqsModel, VpcLatticeModel,
        VpcLatticeV2Model,
    };

    #[cfg(feature = "streaming")]
    pub use aws_lambda_powertools_streaming::{
        BytesRangeSource, RangeSource, S3GetObjectRangeRequest, S3HeadObjectOutput,
        S3HeadObjectRequest, S3ObjectClient, S3ObjectIdentifier, S3RangeSource, SeekableStream,
        StreamingError, StreamingErrorKind, StreamingResult,
    };

    #[cfg(feature = "streaming-async")]
    pub use aws_lambda_powertools_streaming::{
        AsyncRangeFuture, AsyncRangeSource, AsyncS3ObjectClient, AsyncSeekableStream,
    };

    #[cfg(feature = "streaming-s3")]
    pub use aws_lambda_powertools_streaming::{
        AwsSdkS3AsyncRangeReader, AwsSdkS3ObjectClient, AwsSdkS3RangeReader,
    };

    #[cfg(feature = "streaming-csv")]
    pub use aws_lambda_powertools_streaming::{csv_reader, csv_reader_with_builder};

    #[cfg(feature = "streaming-gzip")]
    pub use aws_lambda_powertools_streaming::gzip_decoder;

    #[cfg(feature = "streaming-zip")]
    pub use aws_lambda_powertools_streaming::zip_archive;

    #[cfg(feature = "tracer")]
    pub use aws_lambda_powertools_tracer::{
        TraceContext, TraceFields, TraceSegment, TraceValue, Tracer, TracerConfig,
        XRAY_TRACE_HEADER_NAME,
    };

    #[cfg(feature = "tracer-xray")]
    pub use aws_lambda_powertools_tracer::{XrayDocumentError, XrayDocumentResult};

    #[cfg(feature = "tracer-xray-daemon")]
    pub use aws_lambda_powertools_tracer::{XrayDaemonClient, XrayDaemonConfig, XrayDaemonError};

    #[cfg(feature = "validation")]
    pub use aws_lambda_powertools_validation::{
        Validate, ValidationError, ValidationErrorKind, ValidationResult, Validator,
    };

    #[cfg(feature = "validation-jsonschema")]
    pub use aws_lambda_powertools_validation::JsonSchemaCache;
}

/// Tracing utility.
#[cfg(feature = "tracer")]
pub mod tracer {
    pub use aws_lambda_powertools_tracer::*;
}

/// Validation utility.
#[cfg(feature = "validation")]
pub mod validation {
    pub use aws_lambda_powertools_validation::*;
}
