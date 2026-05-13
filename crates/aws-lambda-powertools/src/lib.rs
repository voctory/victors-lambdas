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
        CorsConfig, Handler, Method, ParseMethodError, PathParams, Request, Response, Route,
        RouteMatch, Router,
    };

    #[cfg(feature = "event-handler-aws-lambda-events")]
    pub use aws_lambda_powertools_event_handler::{
        ApiGatewayAdapterError, ApiGatewayAdapterResult, request_from_apigw_v1,
        request_from_apigw_v2, response_to_apigw_v1, response_to_apigw_v2,
    };

    #[cfg(feature = "idempotency")]
    pub use aws_lambda_powertools_idempotency::{
        Idempotency, IdempotencyConfig, IdempotencyError, IdempotencyExecutionError,
        IdempotencyKey, IdempotencyOutcome, IdempotencyRecord, IdempotencyResult,
        IdempotencyStatus, IdempotencyStore, IdempotencyStoreError, IdempotencyStoreResult,
        InMemoryIdempotencyStore, hash_payload, key_from_json_pointer, key_from_payload,
    };

    #[cfg(feature = "logger")]
    pub use aws_lambda_powertools_logger::{
        LambdaContextFields, LambdaLogContext, LogEntry, LogFields, LogLevel, LogValue, Logger,
        LoggerConfig,
    };

    #[cfg(feature = "metrics")]
    pub use aws_lambda_powertools_metrics::{
        MetadataValue, Metric, MetricResolution, MetricUnit, Metrics, MetricsConfig, MetricsError,
    };

    #[cfg(feature = "parameters")]
    pub use aws_lambda_powertools_parameters::{
        CachePolicy, InMemoryParameterProvider, Parameter, ParameterProvider, Parameters,
    };

    #[cfg(feature = "parser")]
    pub use aws_lambda_powertools_parser::{EventParser, ParseError, ParseErrorKind, ParsedEvent};

    #[cfg(feature = "tracer")]
    pub use aws_lambda_powertools_tracer::{
        TraceContext, TraceFields, TraceSegment, TraceValue, Tracer, TracerConfig,
    };

    #[cfg(feature = "validation")]
    pub use aws_lambda_powertools_validation::{
        Validate, ValidationError, ValidationErrorKind, ValidationResult, Validator,
    };
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
