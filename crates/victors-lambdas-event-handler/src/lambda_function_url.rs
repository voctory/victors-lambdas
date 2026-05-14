//! Lambda Function URL event adapters.

use std::{error::Error, fmt, str::FromStr};

use aws_lambda_events::event::lambda_function_urls::{
    LambdaFunctionUrlRequest, LambdaFunctionUrlResponse,
};
use base64::Engine;
use http::{HeaderMap, HeaderName, HeaderValue};

use crate::{AsyncRouter, Method, Request, Response, Router};

/// Result returned by Lambda Function URL adapter operations.
pub type LambdaFunctionUrlAdapterResult<T> = Result<T, LambdaFunctionUrlAdapterError>;

/// Error returned by Lambda Function URL adapter operations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LambdaFunctionUrlAdapterError {
    /// The incoming event did not include an HTTP method.
    MissingMethod,
    /// The incoming HTTP method is not supported by this router.
    UnsupportedMethod {
        /// Method token received from Lambda Function URL.
        method: String,
    },
    /// Lambda Function URL marked the body as base64 encoded but decoding failed.
    InvalidBodyEncoding {
        /// Decoding error message.
        message: String,
    },
    /// A request header value was not valid UTF-8.
    InvalidRequestHeaderValue {
        /// Header name.
        name: String,
    },
    /// A response header name is not valid for Lambda Function URL.
    InvalidResponseHeaderName {
        /// Header name.
        name: String,
        /// Header parsing error message.
        message: String,
    },
    /// A response header value is not valid for Lambda Function URL.
    InvalidResponseHeaderValue {
        /// Header name.
        name: String,
        /// Header parsing error message.
        message: String,
    },
}

impl fmt::Display for LambdaFunctionUrlAdapterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingMethod => write!(formatter, "Lambda Function URL event is missing method"),
            Self::UnsupportedMethod { method } => {
                write!(
                    formatter,
                    "unsupported Lambda Function URL HTTP method: {method}"
                )
            }
            Self::InvalidBodyEncoding { message } => {
                write!(
                    formatter,
                    "invalid Lambda Function URL body encoding: {message}"
                )
            }
            Self::InvalidRequestHeaderValue { name } => {
                write!(
                    formatter,
                    "Lambda Function URL request header {name} is not valid UTF-8"
                )
            }
            Self::InvalidResponseHeaderName { name, message } => {
                write!(
                    formatter,
                    "invalid Lambda Function URL response header name {name}: {message}"
                )
            }
            Self::InvalidResponseHeaderValue { name, message } => {
                write!(
                    formatter,
                    "invalid Lambda Function URL response header value for {name}: {message}"
                )
            }
        }
    }
}

impl Error for LambdaFunctionUrlAdapterError {}

/// Converts a Lambda Function URL event into a router request.
///
/// # Errors
///
/// Returns an error when the method is missing or unsupported, a header is not
/// UTF-8, or a base64 body cannot be decoded.
pub fn request_from_lambda_function_url(
    event: &LambdaFunctionUrlRequest,
) -> LambdaFunctionUrlAdapterResult<Request> {
    let method = event
        .request_context
        .http
        .method
        .as_deref()
        .ok_or(LambdaFunctionUrlAdapterError::MissingMethod)
        .and_then(method_from_token)?;
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
    for (name, value) in &event.query_string_parameters {
        request = request.with_query_string_parameter(name, value);
    }

    Ok(request)
}

/// Converts a router response into a Lambda Function URL response.
///
/// # Errors
///
/// Returns an error when a response header cannot be represented as an HTTP
/// header name or value.
pub fn response_to_lambda_function_url(
    response: &Response,
) -> LambdaFunctionUrlAdapterResult<LambdaFunctionUrlResponse> {
    let (body, is_base64_encoded) = function_url_body(response.body());
    let mut cookies = Vec::new();
    let headers = response_headers_without_cookies(response.headers(), &mut cookies)?;

    let mut function_url_response = LambdaFunctionUrlResponse::default();
    function_url_response.status_code = i64::from(response.status_code());
    function_url_response.headers = headers;
    function_url_response.body = body;
    function_url_response.is_base64_encoded = is_base64_encoded;
    function_url_response.cookies = cookies;

    Ok(function_url_response)
}

impl Router {
    /// Handles a Lambda Function URL event and returns a Function URL response.
    ///
    /// # Errors
    ///
    /// Returns an error when request adapter conversion fails, except
    /// unsupported HTTP methods which are returned as `405 Method Not Allowed`.
    /// Also returns an error when response adapter conversion fails.
    pub fn handle_lambda_function_url(
        &self,
        event: &LambdaFunctionUrlRequest,
    ) -> LambdaFunctionUrlAdapterResult<LambdaFunctionUrlResponse> {
        let request = match request_from_lambda_function_url(event) {
            Ok(request) => request,
            Err(LambdaFunctionUrlAdapterError::UnsupportedMethod { .. }) => {
                return response_to_lambda_function_url(&Response::new(405));
            }
            Err(error) => return Err(error),
        };
        response_to_lambda_function_url(&self.handle(request))
    }
}

impl AsyncRouter {
    /// Handles a Lambda Function URL event asynchronously.
    ///
    /// # Errors
    ///
    /// Returns an adapter error when request conversion fails, except
    /// unsupported HTTP methods which are returned as `405 Method Not Allowed`.
    /// Also returns an adapter error when response conversion fails.
    pub async fn handle_lambda_function_url(
        &self,
        event: &LambdaFunctionUrlRequest,
    ) -> LambdaFunctionUrlAdapterResult<LambdaFunctionUrlResponse> {
        let request = match request_from_lambda_function_url(event) {
            Ok(request) => request,
            Err(LambdaFunctionUrlAdapterError::UnsupportedMethod { .. }) => {
                return response_to_lambda_function_url(&Response::new(405));
            }
            Err(error) => return Err(error),
        };
        response_to_lambda_function_url(&self.handle(request).await)
    }
}

fn method_from_token(method: &str) -> LambdaFunctionUrlAdapterResult<Method> {
    Method::from_str(method).map_err(|_| LambdaFunctionUrlAdapterError::UnsupportedMethod {
        method: method.to_owned(),
    })
}

fn body_bytes(
    body: Option<&str>,
    is_base64_encoded: bool,
) -> LambdaFunctionUrlAdapterResult<Vec<u8>> {
    let Some(body) = body else {
        return Ok(Vec::new());
    };

    if is_base64_encoded {
        base64::engine::general_purpose::STANDARD
            .decode(body)
            .map_err(|error| LambdaFunctionUrlAdapterError::InvalidBodyEncoding {
                message: error.to_string(),
            })
    } else {
        Ok(body.as_bytes().to_vec())
    }
}

fn add_headers(
    mut request: Request,
    headers: &HeaderMap,
) -> LambdaFunctionUrlAdapterResult<Request> {
    for (name, value) in headers {
        let value = value.to_str().map_err(|_| {
            LambdaFunctionUrlAdapterError::InvalidRequestHeaderValue {
                name: name.as_str().to_owned(),
            }
        })?;
        request = request.with_header(name.as_str(), value);
    }

    Ok(request)
}

fn response_headers_without_cookies(
    headers: &[(String, String)],
    cookies: &mut Vec<String>,
) -> LambdaFunctionUrlAdapterResult<HeaderMap> {
    let mut output = HeaderMap::new();
    for (name, value) in headers {
        if name.eq_ignore_ascii_case("set-cookie") {
            cookies.push(value.clone());
            continue;
        }

        let header_name = HeaderName::from_str(name).map_err(|error| {
            LambdaFunctionUrlAdapterError::InvalidResponseHeaderName {
                name: name.clone(),
                message: error.to_string(),
            }
        })?;
        let header_value = HeaderValue::from_str(value).map_err(|error| {
            LambdaFunctionUrlAdapterError::InvalidResponseHeaderValue {
                name: name.clone(),
                message: error.to_string(),
            }
        })?;
        output.append(header_name, header_value);
    }

    Ok(output)
}

fn function_url_body(body: &[u8]) -> (Option<String>, bool) {
    if body.is_empty() {
        (None, false)
    } else if let Ok(text) = std::str::from_utf8(body) {
        (Some(text.to_owned()), false)
    } else {
        (
            Some(base64::engine::general_purpose::STANDARD.encode(body)),
            true,
        )
    }
}

#[cfg(test)]
mod tests {
    use aws_lambda_events::event::lambda_function_urls::LambdaFunctionUrlRequest;

    use crate::{
        LambdaFunctionUrlAdapterError, Method, Response, Router, request_from_lambda_function_url,
        response_to_lambda_function_url,
    };

    #[test]
    fn converts_lambda_function_url_request() {
        let mut event = LambdaFunctionUrlRequest::default();
        event.raw_path = Some("/orders/123".to_owned());
        event.request_context.http.method = Some("POST".to_owned());
        event.body = Some("aGVsbG8=".to_owned());
        event.is_base64_encoded = true;
        event
            .headers
            .insert("x-request-id", "req-1".parse().unwrap());
        event.query_string_parameters =
            std::collections::HashMap::from([("debug".to_owned(), "true".to_owned())]);

        let request = request_from_lambda_function_url(&event).expect("request converts");

        assert_eq!(request.method(), Method::Post);
        assert_eq!(request.path(), "/orders/123");
        assert_eq!(request.header("x-request-id"), Some("req-1"));
        assert_eq!(request.query_string_parameter("debug"), Some("true"));
        assert_eq!(request.body(), b"hello");
    }

    #[test]
    fn rejects_missing_method() {
        let event = LambdaFunctionUrlRequest::default();

        assert_eq!(
            request_from_lambda_function_url(&event),
            Err(LambdaFunctionUrlAdapterError::MissingMethod)
        );
    }

    #[test]
    fn rejects_invalid_base64_body() {
        let mut event = LambdaFunctionUrlRequest::default();
        event.raw_path = Some("/orders".to_owned());
        event.request_context.http.method = Some("POST".to_owned());
        event.body = Some("not-base64!".to_owned());
        event.is_base64_encoded = true;

        assert!(matches!(
            request_from_lambda_function_url(&event),
            Err(LambdaFunctionUrlAdapterError::InvalidBodyEncoding { .. })
        ));
    }

    #[test]
    fn converts_response_to_lambda_function_url() {
        let response = Response::ok("created").with_header("content-type", "text/plain");

        let function_url_response =
            response_to_lambda_function_url(&response).expect("response converts");

        assert_eq!(function_url_response.status_code, 200);
        assert_eq!(
            function_url_response.headers.get("content-type").unwrap(),
            "text/plain"
        );
        assert_eq!(function_url_response.body, Some("created".to_owned()));
        assert!(!function_url_response.is_base64_encoded);
    }

    #[test]
    fn converts_binary_response_to_base64_lambda_function_url_body() {
        let response = Response::new(200).with_body([0xff, 0x00]);

        let function_url_response =
            response_to_lambda_function_url(&response).expect("response converts");

        assert_eq!(function_url_response.body, Some("/wA=".to_owned()));
        assert!(function_url_response.is_base64_encoded);
    }

    #[test]
    fn converts_set_cookie_headers_to_cookies() {
        let response = Response::ok("ok")
            .with_header("set-cookie", "a=1")
            .with_header("x-request-id", "req-1");

        let function_url_response =
            response_to_lambda_function_url(&response).expect("response converts");

        assert_eq!(function_url_response.cookies, vec!["a=1"]);
        assert_eq!(
            function_url_response.headers.get("x-request-id").unwrap(),
            "req-1"
        );
        assert!(function_url_response.headers.get("set-cookie").is_none());
    }

    #[test]
    fn router_handles_lambda_function_url_events() {
        let mut router = Router::new();
        router.get("/orders/{order_id}", |request| {
            Response::ok(request.path_param("order_id").unwrap_or_default())
        });
        let mut event = LambdaFunctionUrlRequest::default();
        event.raw_path = Some("/orders/123".to_owned());
        event.request_context.http.method = Some("GET".to_owned());

        let response = router
            .handle_lambda_function_url(&event)
            .expect("router handles event");

        assert_eq!(response.status_code, 200);
        assert_eq!(response.body, Some("123".to_owned()));
    }

    #[test]
    fn router_returns_method_not_allowed_for_unsupported_lambda_function_url_methods() {
        let router = Router::new();
        let mut event = LambdaFunctionUrlRequest::default();
        event.request_context.http.method = Some("TRACE".to_owned());

        let response = router
            .handle_lambda_function_url(&event)
            .expect("unsupported method returns a response");

        assert_eq!(response.status_code, 405);
        assert_eq!(response.body, None);
    }
}
