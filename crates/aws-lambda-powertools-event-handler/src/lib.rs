//! Event handler utility.

#[cfg(feature = "aws-lambda-events")]
mod alb;
#[cfg(feature = "aws-lambda-events")]
mod apigateway;
#[cfg(feature = "aws-lambda-events")]
mod appsync;
#[cfg(feature = "appsync-events")]
mod appsync_events;
#[cfg(feature = "aws-lambda-events")]
mod bedrock;
#[cfg(feature = "bedrock-agent-functions")]
mod bedrock_function;
#[cfg(feature = "compression")]
mod compression;
mod cors;
#[cfg(feature = "aws-lambda-events")]
mod lambda_function_url;
mod method;
mod request;
mod response;
mod route;
mod router;
#[cfg(feature = "validation")]
mod validation;
#[cfg(feature = "aws-lambda-events")]
mod vpc_lattice;

#[cfg(feature = "aws-lambda-events")]
pub use alb::{AlbAdapterError, AlbAdapterResult, request_from_alb, response_to_alb};
#[cfg(feature = "aws-lambda-events")]
pub use apigateway::{
    ApiGatewayAdapterError, ApiGatewayAdapterResult, request_from_apigw_v1, request_from_apigw_v2,
    request_from_apigw_websocket, response_to_apigw_v1, response_to_apigw_v2,
    response_to_apigw_websocket,
};
#[cfg(feature = "aws-lambda-events")]
pub use appsync::{
    AppSyncBatchHandler, AppSyncBatchResponseFuture, AppSyncBatchRoute, AppSyncEvent,
    AppSyncHandler, AppSyncResolver, AppSyncResolverError, AppSyncResolverResult,
    AppSyncResponseFuture, AppSyncRoute, AsyncAppSyncBatchHandler, AsyncAppSyncBatchRoute,
    AsyncAppSyncHandler, AsyncAppSyncResolver, AsyncAppSyncRoute,
};
#[cfg(feature = "appsync-events")]
pub use appsync_events::{
    AppSyncEventsAggregatePublishHandler, AppSyncEventsHandlerError, AppSyncEventsHandlerResult,
    AppSyncEventsPublishHandler, AppSyncEventsPublishRoute, AppSyncEventsResolver,
    AppSyncEventsResolverError, AppSyncEventsResolverResult, AppSyncEventsSubscribeHandler,
    AppSyncEventsSubscribeRoute,
};
#[cfg(feature = "aws-lambda-events")]
pub use bedrock::{
    BedrockAgentAdapterError, BedrockAgentAdapterResult, request_from_bedrock_agent,
    response_to_bedrock_agent,
};
#[cfg(feature = "bedrock-agent-functions")]
pub use bedrock_function::{
    AsyncBedrockAgentFunctionHandler, AsyncBedrockAgentFunctionResolver, AsyncBedrockFunctionRoute,
    BedrockAgentFunctionAgent, BedrockAgentFunctionEvent, BedrockAgentFunctionHandler,
    BedrockAgentFunctionHandlerError, BedrockAgentFunctionHandlerResult,
    BedrockAgentFunctionParameter, BedrockAgentFunctionParameterValue,
    BedrockAgentFunctionParameters, BedrockAgentFunctionResolver,
    BedrockAgentFunctionResponseFuture, BedrockAgentFunctionResponseState, BedrockFunctionResponse,
    BedrockFunctionResult, BedrockFunctionRoute,
};
#[cfg(feature = "compression")]
pub use compression::{
    CompressionConfig, CompressionEncoding, DEFAULT_COMPRESSION_THRESHOLD, compress_response,
    compression_middleware,
};
pub use cors::CorsConfig;
#[cfg(feature = "aws-lambda-events")]
pub use lambda_function_url::{
    LambdaFunctionUrlAdapterError, LambdaFunctionUrlAdapterResult,
    request_from_lambda_function_url, response_to_lambda_function_url,
};
pub use method::{Method, ParseMethodError};
pub use request::Request;
pub use response::Response;
pub use route::{
    AsyncFallibleHandler, AsyncHandler, AsyncRoute, FallibleHandler, FallibleResponseFuture,
    Handler, PathParams, ResponseFuture, Route, RouteError, RouteResult,
};
pub use router::{
    AsyncErrorHandler, AsyncRouteMatch, AsyncRouter, ErrorHandler, RequestMiddleware,
    ResponseMiddleware, RouteMatch, Router,
};
#[cfg(feature = "validation")]
pub use validation::{RequestValidator, ResponseValidator, ValidationConfig};
#[cfg(feature = "aws-lambda-events")]
pub use vpc_lattice::{
    VpcLatticeAdapterError, VpcLatticeAdapterResult, request_from_vpc_lattice,
    request_from_vpc_lattice_v2, response_to_vpc_lattice,
};
