//! Event handler utility.

#[cfg(feature = "aws-lambda-events")]
mod apigateway;
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
pub use cors::CorsConfig;
pub use method::{Method, ParseMethodError};
pub use request::Request;
pub use response::Response;
pub use route::{Handler, PathParams, Route};
pub use router::{RequestMiddleware, ResponseMiddleware, RouteMatch, Router};
