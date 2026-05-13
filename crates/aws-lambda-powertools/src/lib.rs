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

/// Common imports for Lambda handlers.
pub mod prelude {
    pub use aws_lambda_powertools_core::{ServiceConfig, ServiceConfigBuilder};

    #[cfg(feature = "batch")]
    pub use aws_lambda_powertools_batch::{
        BatchItemFailure, BatchProcessingReport, BatchProcessor, BatchRecord, BatchRecordResult,
        BatchResponse,
    };

    #[cfg(feature = "event-handler")]
    pub use aws_lambda_powertools_event_handler::{
        AsyncHandler, AsyncRoute, AsyncRouteMatch, AsyncRouter, CorsConfig, Handler, Method,
        ParseMethodError, PathParams, Request, RequestMiddleware, Response, ResponseFuture,
        ResponseMiddleware, Route, RouteMatch, Router,
    };

    #[cfg(feature = "event-handler-validation")]
    pub use aws_lambda_powertools_event_handler::{
        RequestValidator, ResponseValidator, ValidationConfig as EventHandlerValidationConfig,
    };

    #[cfg(feature = "event-handler-aws-lambda-events")]
    pub use aws_lambda_powertools_event_handler::{
        AlbAdapterError, AlbAdapterResult, ApiGatewayAdapterError, ApiGatewayAdapterResult,
        AppSyncEvent, AppSyncHandler, AppSyncResolver, AppSyncResolverError, AppSyncResolverResult,
        AppSyncRoute, BedrockAgentAdapterError, BedrockAgentAdapterResult,
        LambdaFunctionUrlAdapterError, LambdaFunctionUrlAdapterResult, VpcLatticeAdapterError,
        VpcLatticeAdapterResult, request_from_alb, request_from_apigw_v1, request_from_apigw_v2,
        request_from_apigw_websocket, request_from_bedrock_agent, request_from_lambda_function_url,
        request_from_vpc_lattice, request_from_vpc_lattice_v2, response_to_alb,
        response_to_apigw_v1, response_to_apigw_v2, response_to_apigw_websocket,
        response_to_bedrock_agent, response_to_lambda_function_url, response_to_vpc_lattice,
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
        AsyncIdempotency, AsyncIdempotencyStore, Idempotency, IdempotencyConfig, IdempotencyError,
        IdempotencyExecutionError, IdempotencyKey, IdempotencyOutcome, IdempotencyRecord,
        IdempotencyResult, IdempotencyStatus, IdempotencyStore, IdempotencyStoreError,
        IdempotencyStoreFuture, IdempotencyStoreResult, InMemoryIdempotencyStore, hash_payload,
        key_from_json_pointer, key_from_payload,
    };

    #[cfg(feature = "idempotency-dynamodb")]
    pub use aws_lambda_powertools_idempotency::DynamoDbIdempotencyStore;

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
        ParameterProviderError, ParameterProviderResult, ParameterTransformError,
        ParameterTransformErrorKind, Parameters,
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
        CognitoCustomEmailSenderTriggerEvent, CognitoCustomEmailSenderTriggerModel,
        CognitoCustomEmailSenderTriggerSource, CognitoCustomSMSSenderTriggerModel,
        CognitoCustomSenderRequest, CognitoCustomSenderRequestType,
        CognitoCustomSmsSenderTriggerEvent, CognitoCustomSmsSenderTriggerSource,
        CognitoUserPoolCallerContext, DynamoDbStreamBatchInfo, DynamoDbStreamOnFailureDestination,
        DynamoDbStreamRequestContext, DynamoDbStreamResponseContext, EventParser,
        IoTCoreAddOrDeleteFromThingGroupEvent, IoTCoreAddOrRemoveFromThingGroupEvent,
        IoTCorePropagatingAttribute, IoTCoreRegistryCrudOperation, IoTCoreRegistryEventType,
        IoTCoreRegistryMembershipOperation, IoTCoreThingEvent, IoTCoreThingGroupEvent,
        IoTCoreThingGroupHierarchyEvent, IoTCoreThingGroupMembershipEvent,
        IoTCoreThingGroupReference, IoTCoreThingTypeAssociationEvent, IoTCoreThingTypeEvent,
        ParseError, ParseErrorKind, ParsedEvent, TransferFamilyAuthorizerEvent,
        TransferFamilyAuthorizerResponse, TransferFamilyHomeDirectoryEntry,
        TransferFamilyHomeDirectoryType, TransferFamilyPosixProfile, TransferFamilyProtocol,
        TransferFamilyResponseError, TransferFamilyResponseResult,
    };

    #[cfg(feature = "tracer")]
    pub use aws_lambda_powertools_tracer::{
        TraceContext, TraceFields, TraceSegment, TraceValue, Tracer, TracerConfig,
    };

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
