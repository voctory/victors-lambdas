//! Event handler utility.

#[cfg(feature = "aws-lambda-events")]
mod alb;
#[cfg(feature = "aws-lambda-events")]
mod apigateway;
#[cfg(feature = "aws-lambda-events")]
mod appsync;
#[cfg(feature = "aws-lambda-events")]
mod bedrock;
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
    AppSyncEvent, AppSyncHandler, AppSyncResolver, AppSyncResolverError, AppSyncResolverResult,
    AppSyncRoute,
};
#[cfg(feature = "aws-lambda-events")]
pub use bedrock::{
    BedrockAgentAdapterError, BedrockAgentAdapterResult, request_from_bedrock_agent,
    response_to_bedrock_agent,
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
pub use route::{AsyncHandler, AsyncRoute, Handler, PathParams, ResponseFuture, Route};
pub use router::{
    AsyncRouteMatch, AsyncRouter, RequestMiddleware, ResponseMiddleware, RouteMatch, Router,
};
#[cfg(feature = "aws-lambda-events")]
pub use vpc_lattice::{
    VpcLatticeAdapterError, VpcLatticeAdapterResult, request_from_vpc_lattice,
    request_from_vpc_lattice_v2, response_to_vpc_lattice,
};
