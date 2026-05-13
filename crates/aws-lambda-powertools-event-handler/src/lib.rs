//! Event handler utility.

#[cfg(feature = "aws-lambda-events")]
mod apigateway;
#[cfg(feature = "aws-lambda-events")]
mod appsync;
#[cfg(feature = "compression")]
mod compression;
mod cors;
mod method;
mod request;
mod response;
mod route;
mod router;

#[cfg(feature = "aws-lambda-events")]
pub use apigateway::{
    ApiGatewayAdapterError, ApiGatewayAdapterResult, request_from_apigw_v1, request_from_apigw_v2,
    response_to_apigw_v1, response_to_apigw_v2,
};
#[cfg(feature = "aws-lambda-events")]
pub use appsync::{
    AppSyncEvent, AppSyncHandler, AppSyncResolver, AppSyncResolverError, AppSyncResolverResult,
    AppSyncRoute,
};
#[cfg(feature = "compression")]
pub use compression::{
    CompressionConfig, CompressionEncoding, DEFAULT_COMPRESSION_THRESHOLD, compress_response,
    compression_middleware,
};
pub use cors::CorsConfig;
pub use method::{Method, ParseMethodError};
pub use request::Request;
pub use response::Response;
pub use route::{AsyncHandler, AsyncRoute, Handler, PathParams, ResponseFuture, Route};
pub use router::{
    AsyncRouteMatch, AsyncRouter, RequestMiddleware, ResponseMiddleware, RouteMatch, Router,
};
