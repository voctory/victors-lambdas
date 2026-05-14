//! API Gateway event adapters.

use std::{error::Error, fmt, str::FromStr};

use aws_lambda_events::{
    encodings::Body,
    event::apigw::{
        ApiGatewayProxyRequest, ApiGatewayProxyResponse, ApiGatewayV2httpRequest,
        ApiGatewayV2httpResponse, ApiGatewayWebsocketProxyRequest,
    },
};
use base64::Engine;
use http::{HeaderMap, HeaderName, HeaderValue};

use crate::{AsyncRouter, Method, Request, Response, Router};

/// Result returned by API Gateway adapter operations.
pub type ApiGatewayAdapterResult<T> = Result<T, ApiGatewayAdapterError>;

/// Error returned by API Gateway adapter operations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ApiGatewayAdapterError {
    /// The incoming HTTP method is not supported by this router.
    UnsupportedMethod {
        /// Method token received from API Gateway.
        method: String,
    },
    /// API Gateway marked the body as base64 encoded but decoding failed.
    InvalidBodyEncoding {
        /// Decoding error message.
        message: String,
    },
    /// A request header value was not valid UTF-8.
    InvalidRequestHeaderValue {
        /// Header name.
        name: String,
    },
    /// A response header name is not valid for API Gateway.
    InvalidResponseHeaderName {
        /// Header name.
        name: String,
        /// Header parsing error message.
        message: String,
    },
    /// A response header value is not valid for API Gateway.
    InvalidResponseHeaderValue {
        /// Header name.
        name: String,
        /// Header parsing error message.
        message: String,
    },
}

impl fmt::Display for ApiGatewayAdapterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedMethod { method } => {
                write!(formatter, "unsupported API Gateway HTTP method: {method}")
            }
            Self::InvalidBodyEncoding { message } => {
                write!(formatter, "invalid API Gateway body encoding: {message}")
            }
            Self::InvalidRequestHeaderValue { name } => {
                write!(
                    formatter,
                    "API Gateway request header {name} is not valid UTF-8"
                )
            }
            Self::InvalidResponseHeaderName { name, message } => {
                write!(
                    formatter,
                    "invalid API Gateway response header name {name}: {message}"
                )
            }
            Self::InvalidResponseHeaderValue { name, message } => {
                write!(
                    formatter,
                    "invalid API Gateway response header value for {name}: {message}"
                )
            }
        }
    }
}

impl Error for ApiGatewayAdapterError {}

/// Converts an API Gateway REST API v1 event into a router request.
///
/// # Errors
///
/// Returns an error when the method is unsupported, a header is not UTF-8, or a
/// base64 body cannot be decoded.
pub fn request_from_apigw_v1(event: &ApiGatewayProxyRequest) -> ApiGatewayAdapterResult<Request> {
    let method = method_from_token(event.http_method.as_str())?;
    let path = event.path.as_deref().unwrap_or("/");
    let mut request = Request::new(method, path)
        .with_body(body_bytes(event.body.as_deref(), event.is_base64_encoded)?);

    request = add_headers(request, &event.headers)?;
    for (name, value) in event.query_string_parameters.iter() {
        request = request.with_query_string_parameter(name, value);
    }
    for (name, value) in &event.path_parameters {
        request = request.with_path_param(name, value);
    }

    Ok(request)
}

/// Converts an API Gateway HTTP API v2 event into a router request.
///
/// # Errors
///
/// Returns an error when the method is unsupported, a header is not UTF-8, or a
/// base64 body cannot be decoded.
pub fn request_from_apigw_v2(event: &ApiGatewayV2httpRequest) -> ApiGatewayAdapterResult<Request> {
    let method = method_from_token(event.request_context.http.method.as_str())?;
    let path = event
        .raw_path
        .as_deref()
        .or(event.request_context.http.path.as_deref())
        .unwrap_or("/");
    let mut request = Request::new(method, path)
        .with_body(body_bytes(event.body.as_deref(), event.is_base64_encoded)?);

    request = add_headers(request, &event.headers)?;
    for cookie in event.cookies.iter().flatten() {
        request = request.with_header("cookie", cookie);
    }
    for (name, value) in event.query_string_parameters.iter() {
        request = request.with_query_string_parameter(name, value);
    }
    for (name, value) in &event.path_parameters {
        request = request.with_path_param(name, value);
    }

    Ok(request)
}

/// Converts an API Gateway WebSocket API event into a router request.
///
/// WebSocket APIs route by route key instead of a URL path. When a route key is
/// present, this adapter exposes it as a router path such as `/$connect`,
/// `/$disconnect`, `/$default`, or `/sendMessage`.
///
/// # Errors
///
/// Returns an error when the method is unsupported, a header is not UTF-8, or a
/// base64 body cannot be decoded.
pub fn request_from_apigw_websocket(
    event: &ApiGatewayWebsocketProxyRequest,
) -> ApiGatewayAdapterResult<Request> {
    let method = websocket_method(event)?;
    let mut request = Request::new(method, websocket_route_path(event))
        .with_body(body_bytes(event.body.as_deref(), event.is_base64_encoded)?);

    request = add_headers(request, &event.headers)?;
    for (name, value) in event.query_string_parameters.iter() {
        request = request.with_query_string_parameter(name, value);
    }
    for (name, value) in &event.path_parameters {
        request = request.with_path_param(name, value);
    }

    Ok(request)
}

/// Converts a router response into an API Gateway REST API v1 response.
///
/// # Errors
///
/// Returns an error when a response header cannot be represented as an HTTP
/// header name or value.
pub fn response_to_apigw_v1(
    response: &Response,
) -> ApiGatewayAdapterResult<ApiGatewayProxyResponse> {
    let (body, is_base64_encoded) = api_gateway_body(response.body());

    let mut gateway_response = ApiGatewayProxyResponse::default();
    gateway_response.status_code = i64::from(response.status_code());
    gateway_response.headers = response_headers(response.headers())?;
    gateway_response.multi_value_headers = HeaderMap::new();
    gateway_response.body = body;
    gateway_response.is_base64_encoded = is_base64_encoded;

    Ok(gateway_response)
}

/// Converts a router response into an API Gateway WebSocket API response.
///
/// WebSocket Lambda integrations use the same proxy response payload shape as
/// API Gateway REST API proxy integrations.
///
/// # Errors
///
/// Returns an error when a response header cannot be represented as an HTTP
/// header name or value.
pub fn response_to_apigw_websocket(
    response: &Response,
) -> ApiGatewayAdapterResult<ApiGatewayProxyResponse> {
    response_to_apigw_v1(response)
}

/// Converts a router response into an API Gateway HTTP API v2 response.
///
/// # Errors
///
/// Returns an error when a response header cannot be represented as an HTTP
/// header name or value.
pub fn response_to_apigw_v2(
    response: &Response,
) -> ApiGatewayAdapterResult<ApiGatewayV2httpResponse> {
    let (body, is_base64_encoded) = api_gateway_body(response.body());
    let mut cookies = Vec::new();
    let headers = response_headers_without_cookies(response.headers(), &mut cookies)?;

    let mut gateway_response = ApiGatewayV2httpResponse::default();
    gateway_response.status_code = i64::from(response.status_code());
    gateway_response.headers = headers;
    gateway_response.multi_value_headers = HeaderMap::new();
    gateway_response.body = body;
    gateway_response.is_base64_encoded = is_base64_encoded;
    gateway_response.cookies = cookies;

    Ok(gateway_response)
}

impl Router {
    /// Handles an API Gateway REST API v1 event and returns a v1 proxy response.
    ///
    /// # Errors
    ///
    /// Returns an error when request adapter conversion fails, except
    /// unsupported HTTP methods which are returned as `405 Method Not Allowed`.
    /// Also returns an error when response adapter conversion fails.
    pub fn handle_apigw_v1(
        &self,
        event: &ApiGatewayProxyRequest,
    ) -> ApiGatewayAdapterResult<ApiGatewayProxyResponse> {
        let request = match request_from_apigw_v1(event) {
            Ok(request) => request,
            Err(ApiGatewayAdapterError::UnsupportedMethod { .. }) => {
                return response_to_apigw_v1(&Response::new(405));
            }
            Err(error) => return Err(error),
        };
        response_to_apigw_v1(&self.handle(request))
    }

    /// Handles an API Gateway HTTP API v2 event and returns a v2 proxy response.
    ///
    /// # Errors
    ///
    /// Returns an error when request adapter conversion fails, except
    /// unsupported HTTP methods which are returned as `405 Method Not Allowed`.
    /// Also returns an error when response adapter conversion fails.
    pub fn handle_apigw_v2(
        &self,
        event: &ApiGatewayV2httpRequest,
    ) -> ApiGatewayAdapterResult<ApiGatewayV2httpResponse> {
        let request = match request_from_apigw_v2(event) {
            Ok(request) => request,
            Err(ApiGatewayAdapterError::UnsupportedMethod { .. }) => {
                return response_to_apigw_v2(&Response::new(405));
            }
            Err(error) => return Err(error),
        };
        response_to_apigw_v2(&self.handle(request))
    }

    /// Handles an API Gateway WebSocket API event and returns a proxy response.
    ///
    /// # Errors
    ///
    /// Returns an error when request adapter conversion fails, except
    /// unsupported HTTP methods which are returned as `405 Method Not Allowed`.
    /// Also returns an error when response adapter conversion fails.
    pub fn handle_apigw_websocket(
        &self,
        event: &ApiGatewayWebsocketProxyRequest,
    ) -> ApiGatewayAdapterResult<ApiGatewayProxyResponse> {
        let request = match request_from_apigw_websocket(event) {
            Ok(request) => request,
            Err(ApiGatewayAdapterError::UnsupportedMethod { .. }) => {
                return response_to_apigw_websocket(&Response::new(405));
            }
            Err(error) => return Err(error),
        };
        response_to_apigw_websocket(&self.handle(request))
    }
}

impl AsyncRouter {
    /// Handles an API Gateway REST API v1 request asynchronously.
    ///
    /// # Errors
    ///
    /// Returns an adapter error when request conversion fails, except
    /// unsupported HTTP methods which are returned as `405 Method Not Allowed`.
    /// Also returns an adapter error when response conversion fails.
    pub async fn handle_apigw_v1(
        &self,
        event: &ApiGatewayProxyRequest,
    ) -> ApiGatewayAdapterResult<ApiGatewayProxyResponse> {
        let request = match request_from_apigw_v1(event) {
            Ok(request) => request,
            Err(ApiGatewayAdapterError::UnsupportedMethod { .. }) => {
                return response_to_apigw_v1(&Response::new(405));
            }
            Err(error) => return Err(error),
        };
        response_to_apigw_v1(&self.handle(request).await)
    }

    /// Handles an API Gateway HTTP API v2 request asynchronously.
    ///
    /// # Errors
    ///
    /// Returns an adapter error when request conversion fails, except
    /// unsupported HTTP methods which are returned as `405 Method Not Allowed`.
    /// Also returns an adapter error when response conversion fails.
    pub async fn handle_apigw_v2(
        &self,
        event: &ApiGatewayV2httpRequest,
    ) -> ApiGatewayAdapterResult<ApiGatewayV2httpResponse> {
        let request = match request_from_apigw_v2(event) {
            Ok(request) => request,
            Err(ApiGatewayAdapterError::UnsupportedMethod { .. }) => {
                return response_to_apigw_v2(&Response::new(405));
            }
            Err(error) => return Err(error),
        };
        response_to_apigw_v2(&self.handle(request).await)
    }

    /// Handles an API Gateway WebSocket API event asynchronously.
    ///
    /// # Errors
    ///
    /// Returns an adapter error when request conversion fails, except
    /// unsupported HTTP methods which are returned as `405 Method Not Allowed`.
    /// Also returns an adapter error when response conversion fails.
    pub async fn handle_apigw_websocket(
        &self,
        event: &ApiGatewayWebsocketProxyRequest,
    ) -> ApiGatewayAdapterResult<ApiGatewayProxyResponse> {
        let request = match request_from_apigw_websocket(event) {
            Ok(request) => request,
            Err(ApiGatewayAdapterError::UnsupportedMethod { .. }) => {
                return response_to_apigw_websocket(&Response::new(405));
            }
            Err(error) => return Err(error),
        };
        response_to_apigw_websocket(&self.handle(request).await)
    }
}

fn method_from_token(method: &str) -> ApiGatewayAdapterResult<Method> {
    Method::from_str(method).map_err(|_| ApiGatewayAdapterError::UnsupportedMethod {
        method: method.to_owned(),
    })
}

fn body_bytes(body: Option<&str>, is_base64_encoded: bool) -> ApiGatewayAdapterResult<Vec<u8>> {
    let Some(body) = body else {
        return Ok(Vec::new());
    };

    if is_base64_encoded {
        base64::engine::general_purpose::STANDARD
            .decode(body)
            .map_err(|error| ApiGatewayAdapterError::InvalidBodyEncoding {
                message: error.to_string(),
            })
    } else {
        Ok(body.as_bytes().to_vec())
    }
}

fn websocket_method(event: &ApiGatewayWebsocketProxyRequest) -> ApiGatewayAdapterResult<Method> {
    if let Some(method) = event
        .http_method
        .as_ref()
        .or(event.request_context.http_method.as_ref())
    {
        method_from_token(method.as_str())
    } else {
        Ok(Method::Get)
    }
}

fn websocket_route_path(event: &ApiGatewayWebsocketProxyRequest) -> String {
    event
        .request_context
        .route_key
        .as_deref()
        .or(event.request_context.resource_path.as_deref())
        .or(event.path.as_deref())
        .or(event.resource.as_deref())
        .map_or_else(|| "/$default".to_owned(), router_path)
}

fn router_path(path: &str) -> String {
    if path.is_empty() {
        "/".to_owned()
    } else if path.starts_with('/') {
        path.to_owned()
    } else {
        format!("/{path}")
    }
}

fn add_headers(mut request: Request, headers: &HeaderMap) -> ApiGatewayAdapterResult<Request> {
    for (name, value) in headers {
        let value =
            value
                .to_str()
                .map_err(|_| ApiGatewayAdapterError::InvalidRequestHeaderValue {
                    name: name.as_str().to_owned(),
                })?;
        request = request.with_header(name.as_str(), value);
    }

    Ok(request)
}

fn response_headers(headers: &[(String, String)]) -> ApiGatewayAdapterResult<HeaderMap> {
    let mut output = HeaderMap::new();
    append_headers(headers, &mut output, None)?;
    Ok(output)
}

fn response_headers_without_cookies(
    headers: &[(String, String)],
    cookies: &mut Vec<String>,
) -> ApiGatewayAdapterResult<HeaderMap> {
    let mut output = HeaderMap::new();
    append_headers(headers, &mut output, Some(cookies))?;
    Ok(output)
}

fn append_headers(
    headers: &[(String, String)],
    output: &mut HeaderMap,
    mut cookies: Option<&mut Vec<String>>,
) -> ApiGatewayAdapterResult<()> {
    for (name, value) in headers {
        if name.eq_ignore_ascii_case("set-cookie") {
            if let Some(cookies) = cookies.as_deref_mut() {
                cookies.push(value.clone());
                continue;
            }
        }

        let header_name = HeaderName::from_str(name).map_err(|error| {
            ApiGatewayAdapterError::InvalidResponseHeaderName {
                name: name.clone(),
                message: error.to_string(),
            }
        })?;
        let header_value = HeaderValue::from_str(value).map_err(|error| {
            ApiGatewayAdapterError::InvalidResponseHeaderValue {
                name: name.clone(),
                message: error.to_string(),
            }
        })?;
        output.append(header_name, header_value);
    }

    Ok(())
}

fn api_gateway_body(body: &[u8]) -> (Option<Body>, bool) {
    if body.is_empty() {
        (None, false)
    } else if let Ok(text) = std::str::from_utf8(body) {
        (Some(Body::Text(text.to_owned())), false)
    } else {
        (Some(Body::Binary(body.to_vec())), true)
    }
}

#[cfg(test)]
mod tests {
    use aws_lambda_events::{
        encodings::Body,
        event::apigw::{
            ApiGatewayProxyRequest, ApiGatewayV2httpRequest, ApiGatewayWebsocketProxyRequest,
        },
    };
    use http::Method as HttpMethod;

    use crate::{
        ApiGatewayAdapterError, Method, Request, Response, Router, request_from_apigw_v1,
        request_from_apigw_websocket, response_to_apigw_v1, response_to_apigw_v2,
        response_to_apigw_websocket,
    };

    #[test]
    fn converts_api_gateway_v1_request() {
        let mut event = ApiGatewayProxyRequest::default();
        event.path = Some("/orders/123".to_owned());
        event.http_method = HttpMethod::POST;
        event.body = Some("aGVsbG8=".to_owned());
        event.is_base64_encoded = true;
        event
            .headers
            .insert("x-request-id", "req-1".parse().unwrap());
        event.query_string_parameters =
            std::collections::HashMap::from([("debug".to_owned(), "true".to_owned())]).into();
        event
            .path_parameters
            .insert("order_id".to_owned(), "123".to_owned());

        let request = request_from_apigw_v1(&event).expect("request converts");

        assert_eq!(request.method(), Method::Post);
        assert_eq!(request.path(), "/orders/123");
        assert_eq!(request.header("x-request-id"), Some("req-1"));
        assert_eq!(request.query_string_parameter("debug"), Some("true"));
        assert_eq!(request.path_param("order_id"), Some("123"));
        assert_eq!(request.body(), b"hello");
    }

    #[test]
    fn rejects_invalid_base64_body() {
        let mut event = ApiGatewayProxyRequest::default();
        event.path = Some("/orders".to_owned());
        event.http_method = HttpMethod::POST;
        event.body = Some("not-base64!".to_owned());
        event.is_base64_encoded = true;

        assert!(matches!(
            request_from_apigw_v1(&event),
            Err(ApiGatewayAdapterError::InvalidBodyEncoding { .. })
        ));
    }

    #[test]
    fn converts_response_to_api_gateway_v1() {
        let response = Response::ok("created").with_header("content-type", "text/plain");

        let gateway_response = response_to_apigw_v1(&response).expect("response converts");

        assert_eq!(gateway_response.status_code, 200);
        assert_eq!(
            gateway_response.headers.get("content-type").unwrap(),
            "text/plain"
        );
        assert_eq!(
            gateway_response.body,
            Some(Body::Text("created".to_owned()))
        );
        assert!(!gateway_response.is_base64_encoded);
    }

    #[test]
    fn converts_api_gateway_websocket_request() {
        let mut event = ApiGatewayWebsocketProxyRequest::default();
        event.request_context.route_key = Some("sendMessage".to_owned());
        event.body = Some("hello".to_owned());
        event
            .headers
            .insert("x-request-id", "req-1".parse().unwrap());
        event.query_string_parameters =
            std::collections::HashMap::from([("debug".to_owned(), "true".to_owned())]).into();
        event
            .path_parameters
            .insert("connection_id".to_owned(), "abc123".to_owned());

        let request = request_from_apigw_websocket(&event).expect("request converts");

        assert_eq!(request.method(), Method::Get);
        assert_eq!(request.path(), "/sendMessage");
        assert_eq!(request.header("x-request-id"), Some("req-1"));
        assert_eq!(request.query_string_parameter("debug"), Some("true"));
        assert_eq!(request.path_param("connection_id"), Some("abc123"));
        assert_eq!(request.body(), b"hello");
    }

    #[test]
    fn converts_response_to_api_gateway_websocket() {
        let response = Response::ok("accepted").with_header("content-type", "text/plain");

        let gateway_response = response_to_apigw_websocket(&response).expect("response converts");

        assert_eq!(gateway_response.status_code, 200);
        assert_eq!(
            gateway_response.headers.get("content-type").unwrap(),
            "text/plain"
        );
        assert_eq!(
            gateway_response.body,
            Some(Body::Text("accepted".to_owned()))
        );
        assert!(!gateway_response.is_base64_encoded);
    }

    #[test]
    fn converts_binary_response_to_base64_api_gateway_body() {
        let response = Response::new(200).with_body([0xff, 0x00]);

        let gateway_response = response_to_apigw_v1(&response).expect("response converts");

        assert_eq!(gateway_response.body, Some(Body::Binary(vec![0xff, 0x00])));
        assert!(gateway_response.is_base64_encoded);
    }

    #[test]
    fn converts_set_cookie_headers_to_v2_cookies() {
        let response = Response::ok("ok")
            .with_header("set-cookie", "a=1")
            .with_header("x-request-id", "req-1");

        let gateway_response = response_to_apigw_v2(&response).expect("response converts");

        assert_eq!(gateway_response.cookies, vec!["a=1"]);
        assert_eq!(
            gateway_response.headers.get("x-request-id").unwrap(),
            "req-1"
        );
        assert!(gateway_response.headers.get("set-cookie").is_none());
    }

    #[test]
    fn router_handles_api_gateway_v2_events() {
        let mut router = Router::new();
        router.get("/orders/{order_id}", |request| {
            Response::ok(request.path_param("order_id").unwrap_or_default())
        });
        let mut event = ApiGatewayV2httpRequest::default();
        event.raw_path = Some("/orders/123".to_owned());
        event.request_context.http.method = HttpMethod::GET;

        let response = router
            .handle_apigw_v2(&event)
            .expect("router handles event");

        assert_eq!(response.status_code, 200);
        assert_eq!(response.body, Some(Body::Text("123".to_owned())));
    }

    #[test]
    fn router_returns_method_not_allowed_for_unsupported_api_gateway_methods() {
        let router = Router::new();
        let mut v1_event = ApiGatewayProxyRequest::default();
        v1_event.http_method = HttpMethod::TRACE;
        let mut v2_event = ApiGatewayV2httpRequest::default();
        v2_event.request_context.http.method = HttpMethod::CONNECT;
        let mut websocket_event = ApiGatewayWebsocketProxyRequest::default();
        websocket_event.http_method = Some(HttpMethod::TRACE);

        let v1_response = router
            .handle_apigw_v1(&v1_event)
            .expect("unsupported method returns a response");
        let v2_response = router
            .handle_apigw_v2(&v2_event)
            .expect("unsupported method returns a response");
        let websocket_response = router
            .handle_apigw_websocket(&websocket_event)
            .expect("unsupported method returns a response");

        assert_eq!(v1_response.status_code, 405);
        assert_eq!(v1_response.body, None);
        assert_eq!(v2_response.status_code, 405);
        assert_eq!(v2_response.body, None);
        assert_eq!(websocket_response.status_code, 405);
        assert_eq!(websocket_response.body, None);
    }

    #[test]
    fn router_handles_api_gateway_websocket_events() {
        let mut router = Router::new();
        router.any("/sendMessage", |request| {
            Response::ok(std::str::from_utf8(request.body()).unwrap_or_default())
        });
        let mut event = ApiGatewayWebsocketProxyRequest::default();
        event.request_context.route_key = Some("sendMessage".to_owned());
        event.body = Some("ping".to_owned());

        let response = router
            .handle_apigw_websocket(&event)
            .expect("router handles event");

        assert_eq!(response.status_code, 200);
        assert_eq!(response.body, Some(Body::Text("ping".to_owned())));
    }

    #[test]
    fn manual_request_path_params_can_be_added() {
        let request = Request::new(Method::Get, "/orders/123").with_path_param("order_id", "123");

        assert_eq!(request.path_param("order_id"), Some("123"));
    }
}
