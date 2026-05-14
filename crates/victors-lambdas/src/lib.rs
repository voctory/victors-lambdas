//! Umbrella crate for Victor's Lambdas utilities.

/// Batch processing utility.
#[cfg(feature = "batch")]
pub mod batch {
    pub use victors_lambdas_batch::*;
}

/// Shared configuration and runtime helpers.
pub mod core {
    pub use victors_lambdas_core::*;
}

/// Data masking utility.
#[cfg(feature = "data-masking")]
pub mod data_masking {
    pub use victors_lambdas_data_masking::*;
}

/// Event handler utility.
#[cfg(feature = "event-handler")]
pub mod event_handler {
    pub use victors_lambdas_event_handler::*;
}

/// Feature flag evaluation utility.
#[cfg(feature = "feature-flags")]
pub mod feature_flags {
    pub use victors_lambdas_feature_flags::*;
}

/// Idempotency utility.
#[cfg(feature = "idempotency")]
pub mod idempotency {
    pub use victors_lambdas_idempotency::*;
}

/// `JMESPath` extraction utility.
#[cfg(feature = "jmespath")]
pub mod jmespath {
    pub use victors_lambdas_jmespath::*;
}

/// Kafka consumer record utility.
#[cfg(feature = "kafka")]
pub mod kafka {
    pub use victors_lambdas_kafka::*;
}

/// Structured logging utility.
#[cfg(feature = "logger")]
pub mod logger {
    pub use victors_lambdas_logger::*;
}

/// Lambda execution environment metadata utility.
#[cfg(feature = "metadata")]
pub mod metadata {
    pub use victors_lambdas_metadata::*;
}

/// Metrics utility.
#[cfg(feature = "metrics")]
pub mod metrics {
    pub use victors_lambdas_metrics::*;
}

/// Parameter retrieval utility.
#[cfg(feature = "parameters")]
pub mod parameters {
    pub use victors_lambdas_parameters::*;
}

/// Event parsing utility.
#[cfg(feature = "parser")]
pub mod parser {
    pub use victors_lambdas_parser::*;
}

/// Seekable streaming utility.
#[cfg(feature = "streaming")]
pub mod streaming {
    pub use victors_lambdas_streaming::*;
}

/// Common imports for Lambda handlers.
pub mod prelude {
    pub use victors_lambdas_core::{ServiceConfig, ServiceConfigBuilder};

    #[cfg(feature = "batch")]
    pub use victors_lambdas_batch::{
        BatchItemFailure, BatchProcessingReport, BatchProcessor, BatchRecord, BatchRecordResult,
        BatchResponse,
    };

    #[cfg(feature = "batch-parser")]
    pub use victors_lambdas_batch::ParsedBatchRecord;

    #[cfg(feature = "batch-aws-lambda-events")]
    pub use victors_lambdas_batch::{
        KafkaBatchItemFailure, KafkaBatchItemIdentifier, KafkaBatchProcessingReport,
        KafkaBatchRecordResult, KafkaBatchResponse,
    };

    #[cfg(feature = "data-masking")]
    pub use victors_lambdas_data_masking::{
        DATA_MASKING_STRING, DataMasking, DataMaskingConfig, DataMaskingError,
        DataMaskingErrorKind, DataMaskingProvider, DataMaskingResult, EncryptionContext,
        MaskingOptions, MaskingStrategy, erase, erase_fields, erase_fields_with_rules,
    };

    #[cfg(feature = "data-masking-kms")]
    pub use victors_lambdas_data_masking::KmsDataMaskingProvider;

    #[cfg(feature = "event-handler")]
    pub use victors_lambdas_event_handler::{
        AsyncErrorHandler, AsyncFallibleHandler, AsyncHandler, AsyncRoute, AsyncRouteMatch,
        AsyncRouter, CorsConfig, ErrorHandler, Extensions, FallibleHandler, FallibleResponseFuture,
        Handler, HttpError, MatchedRoute, Method, ParseMethodError, PathParams, Request,
        RequestMiddleware, Response, ResponseFuture, ResponseMiddleware, Route, RouteError,
        RouteMatch, RouteResult, Router,
    };

    #[cfg(feature = "event-handler-validation")]
    pub use victors_lambdas_event_handler::{
        RequestValidator, ResponseValidator, ValidationConfig as EventHandlerValidationConfig,
    };

    #[cfg(feature = "event-handler-metrics")]
    pub use victors_lambdas_event_handler::{
        HttpMetricsResult, http_metrics_response_middleware, http_metrics_start_middleware,
        record_http_metrics,
    };

    #[cfg(feature = "event-handler-tracer")]
    pub use victors_lambdas_event_handler::{
        HttpTraceConfig, HttpTraceSink, http_trace_response_middleware,
        http_trace_response_middleware_with_config, record_http_trace,
    };

    #[cfg(feature = "event-handler-aws-lambda-events")]
    pub use victors_lambdas_event_handler::{
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

    #[cfg(feature = "event-handler-appsync-scalars")]
    pub use victors_lambdas_event_handler::{
        AppSyncScalarError, AppSyncScalarResult, AppSyncTimeOffset, aws_date, aws_date_time,
        aws_time, aws_timestamp, make_id,
    };

    #[cfg(feature = "event-handler-appsync-events")]
    pub use victors_lambdas_event_handler::{
        AppSyncEventsAggregatePublishHandler, AppSyncEventsHandlerError,
        AppSyncEventsHandlerResult, AppSyncEventsPublishHandler, AppSyncEventsPublishRoute,
        AppSyncEventsResolver, AppSyncEventsResolverError, AppSyncEventsResolverResult,
        AppSyncEventsSubscribeHandler, AppSyncEventsSubscribeRoute,
    };

    #[cfg(feature = "event-handler-bedrock-agent-functions")]
    pub use victors_lambdas_event_handler::{
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
    pub use victors_lambdas_feature_flags::{
        AsyncFeatureFlagStore, AsyncFeatureFlags, FeatureCondition, FeatureFlag,
        FeatureFlagCachePolicy, FeatureFlagConfig, FeatureFlagContext, FeatureFlagError,
        FeatureFlagErrorKind, FeatureFlagFuture, FeatureFlagResult, FeatureFlagStore, FeatureFlags,
        FeatureRule, InMemoryFeatureFlagStore, RuleAction,
    };

    #[cfg(feature = "feature-flags-appconfig")]
    pub use victors_lambdas_feature_flags::AppConfigFeatureFlagStore;

    #[cfg(feature = "idempotency")]
    pub use victors_lambdas_idempotency::{
        AsyncIdempotency, AsyncIdempotencyCacheClient, AsyncIdempotencyStore,
        CacheIdempotencyStore, CachedIdempotencyStore, Idempotency, IdempotencyConfig,
        IdempotencyError, IdempotencyExecutionError, IdempotencyKey, IdempotencyOutcome,
        IdempotencyRecord, IdempotencyResult, IdempotencyStatus, IdempotencyStore,
        IdempotencyStoreError, IdempotencyStoreFuture, IdempotencyStoreResult,
        InMemoryIdempotencyStore, PayloadValidation, hash_payload, key_from_json_pointer,
        key_from_payload,
    };

    #[cfg(feature = "idempotency-dynamodb")]
    pub use victors_lambdas_idempotency::DynamoDbIdempotencyStore;

    #[cfg(feature = "idempotency-jmespath")]
    pub use victors_lambdas_idempotency::{hash_payload_from_jmespath, key_from_jmespath};

    #[cfg(feature = "jmespath")]
    pub use victors_lambdas_jmespath::{
        API_GATEWAY_HTTP, API_GATEWAY_REST, CLOUDWATCH_EVENTS_SCHEDULED, CLOUDWATCH_LOGS,
        EVENTBRIDGE, JmespathError, JmespathErrorKind, JmespathExpression, JmespathResult,
        KINESIS_DATA_STREAM, S3_EVENTBRIDGE_SQS, S3_KINESIS_FIREHOSE, S3_SNS_KINESIS_FIREHOSE,
        S3_SNS_SQS, S3_SQS, SNS, SQS, extract_data_from_envelope, query, search, search_as,
    };

    #[cfg(feature = "kafka")]
    pub use victors_lambdas_kafka::{
        ConsumerRecord, ConsumerRecords, JsonKafkaFieldDecoder, KafkaConsumer, KafkaConsumerConfig,
        KafkaConsumerError, KafkaConsumerErrorKind, KafkaConsumerResult, KafkaField,
        KafkaFieldDecoder, KafkaFieldDeserializer, KafkaSchemaConfig, KafkaSchemaConsumer,
        KafkaSchemaMetadata, KafkaSchemaType, PrimitiveKafkaFieldDecoder, consumer_records,
        decode_base64_json, decode_base64_string, schema_consumer_records,
    };

    #[cfg(feature = "kafka-avro")]
    pub use victors_lambdas_kafka::{AvroKafkaFieldDecoder, decode_base64_avro};

    #[cfg(feature = "kafka-protobuf")]
    pub use victors_lambdas_kafka::{
        ProtobufKafkaFieldDecoder, ProtobufWireFormat, decode_base64_protobuf,
        decode_base64_protobuf_with_format,
    };

    #[cfg(feature = "logger")]
    pub use victors_lambdas_logger::{
        DEFAULT_LOG_BUFFER_KEY, JsonLogFormatter, LambdaContextFields, LambdaLogContext, LogBuffer,
        LogBufferConfig, LogBufferError, LogEntry, LogFields, LogFormatter, LogLevel, LogRedactor,
        LogValue, Logger, LoggerConfig,
    };

    #[cfg(feature = "logger-tracing")]
    pub use victors_lambdas_logger::LoggerLayer;

    #[cfg(feature = "metadata")]
    pub use victors_lambdas_metadata::{
        DEFAULT_LAMBDA_METADATA_TIMEOUT, LAMBDA_METADATA_API_VERSION, LAMBDA_METADATA_PATH,
        LambdaMetadata, LambdaMetadataClient, LambdaMetadataError, LambdaMetadataErrorKind,
        LambdaMetadataResult, clear_lambda_metadata_cache, get_lambda_metadata,
        get_lambda_metadata_with_timeout,
    };

    #[cfg(feature = "metrics")]
    pub use victors_lambdas_metrics::{
        MetadataValue, Metric, MetricResolution, MetricUnit, Metrics, MetricsConfig, MetricsError,
        MetricsFuture,
    };

    #[cfg(feature = "parameters")]
    pub use victors_lambdas_parameters::{
        AsyncParameterError, AsyncParameterProvider, AsyncParameterResult, AsyncParameters,
        CachePolicy, InMemoryParameterProvider, Parameter, ParameterFuture, ParameterProvider,
        ParameterProviderError, ParameterProviderResult, ParameterTransform,
        ParameterTransformError, ParameterTransformErrorKind, ParameterValue, Parameters,
    };

    #[cfg(feature = "parameters-appconfig")]
    pub use victors_lambdas_parameters::AppConfigProvider;

    #[cfg(feature = "parameters-dynamodb")]
    pub use victors_lambdas_parameters::DynamoDbParameterProvider;

    #[cfg(feature = "parameters-secrets")]
    pub use victors_lambdas_parameters::SecretsManagerProvider;

    #[cfg(feature = "parameters-ssm")]
    pub use victors_lambdas_parameters::{
        SsmParameterProvider, SsmParameterType, SsmParametersByName,
    };

    #[cfg(feature = "parser")]
    pub use victors_lambdas_parser::{
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
    pub use victors_lambdas_parser::{
        ActiveMqModel, AlbModel, ApiGatewayAuthorizerHttpApiV1Request,
        ApiGatewayAuthorizerIamPolicyResponse, ApiGatewayAuthorizerRequest,
        ApiGatewayAuthorizerRequestV2, ApiGatewayAuthorizerResponse,
        ApiGatewayAuthorizerSimpleResponse, ApiGatewayAuthorizerToken, ApiGatewayProxyEventModel,
        ApiGatewayProxyEventV2Model, ApiGatewayProxyEventV2RequestContextModel,
        ApiGatewayProxyRequestContextModel, ApiGatewayWebsocketConnectEvent,
        ApiGatewayWebsocketDisconnectEvent, ApiGatewayWebsocketEventModel,
        ApiGatewayWebsocketMessageEvent, AppSyncBatchResolverEvent, AppSyncResolverEvent,
        CloudFormationCustomResourceCreate, CloudFormationCustomResourceDelete,
        CloudFormationCustomResourceRequest, CloudFormationCustomResourceResponse,
        CloudFormationCustomResourceResponseStatus, CloudFormationCustomResourceUpdate,
        CloudWatchLogEventModel, CloudWatchLogsDecodeModel, CloudWatchLogsModel,
        CognitoCreateAuthChallengeTriggerModel, CognitoCustomMessageTriggerModel,
        CognitoDefineAuthChallengeTriggerModel, CognitoPostAuthenticationTriggerModel,
        CognitoPostConfirmationTriggerModel, CognitoPreAuthenticationTriggerModel,
        CognitoPreSignupTriggerModel, CognitoPreTokenGenerationTriggerModelV1,
        CognitoPreTokenGenerationTriggerModelV2AndV3, CognitoVerifyAuthChallengeTriggerModel,
        DynamoDbStreamModel, DynamoDbStreamRecordModel, DynamoDbStreamToKinesisRecordModel,
        EventBridgeModel, KafkaMskEventModel, KafkaRecordModel, KafkaSelfManagedEventModel,
        KinesisDataStreamModel, KinesisDataStreamRecordModel, KinesisDynamoDbStreamModel,
        KinesisFirehoseModel, KinesisFirehoseRecordModel, KinesisFirehoseSqsModel,
        KinesisFirehoseSqsRecordModel, LambdaFunctionUrlModel, RabbitMqModel,
        S3BatchOperationModel, S3Model, S3ObjectLambdaEvent, S3ObjectLambdaModel,
        S3SqsEventNotificationModel, SesModel, SesRecordModel, SnsModel, SnsNotificationModel,
        SnsRecordModel, SnsSqsNotificationModel, SqsMessageAttributeModel, SqsModel,
        SqsRecordModel, VpcLatticeModel, VpcLatticeV2Model,
    };

    #[cfg(feature = "streaming")]
    pub use victors_lambdas_streaming::{
        BytesRangeSource, RangeSource, S3GetObjectRangeRequest, S3HeadObjectOutput,
        S3HeadObjectRequest, S3Object, S3ObjectClient, S3ObjectIdentifier, S3RangeSource,
        SeekableStream, StreamingError, StreamingErrorKind, StreamingResult,
    };

    #[cfg(feature = "streaming-async")]
    pub use victors_lambdas_streaming::{
        AsyncRangeFuture, AsyncRangeSource, AsyncS3ObjectClient, AsyncSeekableStream,
    };

    #[cfg(feature = "streaming-s3")]
    pub use victors_lambdas_streaming::{
        AwsSdkS3AsyncRangeReader, AwsSdkS3ObjectClient, AwsSdkS3RangeReader,
    };

    #[cfg(feature = "streaming-csv")]
    pub use victors_lambdas_streaming::{csv_reader, csv_reader_with_builder};

    #[cfg(feature = "streaming-gzip")]
    pub use victors_lambdas_streaming::gzip_decoder;

    #[cfg(feature = "streaming-zip")]
    pub use victors_lambdas_streaming::zip_archive;

    #[cfg(feature = "tracer")]
    pub use victors_lambdas_tracer::{
        TraceContext, TraceFields, TraceSegment, TraceValue, Tracer, TracerConfig,
        XRAY_TRACE_HEADER_NAME,
    };

    #[cfg(feature = "tracer-xray")]
    pub use victors_lambdas_tracer::{XrayDocumentError, XrayDocumentResult};

    #[cfg(feature = "tracer-xray-daemon")]
    pub use victors_lambdas_tracer::{XrayDaemonClient, XrayDaemonConfig, XrayDaemonError};

    #[cfg(feature = "validation")]
    pub use victors_lambdas_validation::{
        Validate, ValidationError, ValidationErrorKind, ValidationResult, Validator,
    };

    #[cfg(feature = "validation-jmespath")]
    pub use victors_lambdas_validation::extract_envelope as extract_validation_envelope;

    #[cfg(feature = "validation-jsonschema")]
    pub use victors_lambdas_validation::JsonSchemaCache;
}

/// Tracing utility.
#[cfg(feature = "tracer")]
pub mod tracer {
    pub use victors_lambdas_tracer::*;
}

/// Validation utility.
#[cfg(feature = "validation")]
pub mod validation {
    pub use victors_lambdas_validation::*;
}
