//! Application Load Balancer event adapters.

use std::{error::Error, fmt, str::FromStr};

use aws_lambda_events::{
    encodings::Body,
    event::alb::{AlbTargetGroupRequest, AlbTargetGroupResponse},
};
use base64::Engine;
use http::{HeaderMap, HeaderName, HeaderValue};

use crate::{AsyncRouter, Method, Request, Response, Router};

/// Result returned by ALB adapter operations.
pub type AlbAdapterResult<T> = Result<T, AlbAdapterError>;

/// Error returned by ALB adapter operations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AlbAdapterError {
    /// The incoming HTTP method is not supported by this router.
    UnsupportedMethod {
        /// Method token received from ALB.
        method: String,
    },
    /// ALB marked the body as base64 encoded but decoding failed.
    InvalidBodyEncoding {
        /// Decoding error message.
        message: String,
    },
    /// A request header value was not valid UTF-8.
    InvalidRequestHeaderValue {
        /// Header name.
        name: String,
    },
    /// A response header name is not valid for ALB.
    InvalidResponseHeaderName {
        /// Header name.
        name: String,
        /// Header parsing error message.
        message: String,
    },
    /// A response header value is not valid for ALB.
    InvalidResponseHeaderValue {
        /// Header name.
        name: String,
        /// Header parsing error message.
        message: String,
    },
}

impl fmt::Display for AlbAdapterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedMethod { method } => {
                write!(formatter, "unsupported ALB HTTP method: {method}")
            }
            Self::InvalidBodyEncoding { message } => {
                write!(formatter, "invalid ALB body encoding: {message}")
            }
            Self::InvalidRequestHeaderValue { name } => {
                write!(formatter, "ALB request header {name} is not valid UTF-8")
            }
            Self::InvalidResponseHeaderName { name, message } => {
                write!(
                    formatter,
                    "invalid ALB response header name {name}: {message}"
                )
            }
            Self::InvalidResponseHeaderValue { name, message } => {
                write!(
                    formatter,
                    "invalid ALB response header value for {name}: {message}"
                )
            }
        }
    }
}

impl Error for AlbAdapterError {}

/// Converts an ALB Lambda target group event into a router request.
///
/// # Errors
///
/// Returns an error when the method is unsupported, a header is not UTF-8, or a
/// base64 body cannot be decoded.
pub fn request_from_alb(event: &AlbTargetGroupRequest) -> AlbAdapterResult<Request> {
    let method = method_from_token(event.http_method.as_str())?;
    let path = event.path.as_deref().unwrap_or("/");
    let mut request = Request::new(method, path)
        .with_body(body_bytes(event.body.as_deref(), event.is_base64_encoded)?);

    request = add_headers(request, &event.headers)?;
    request = add_headers(request, &event.multi_value_headers)?;
    for (name, value) in event.query_string_parameters.iter() {
        request = request.with_query_string_parameter(name, value);
    }
    for (name, value) in event.multi_value_query_string_parameters.iter() {
        request = request.with_query_string_parameter(name, value);
    }

    Ok(request)
}

/// Converts a router response into an ALB target group response.
///
/// # Errors
///
/// Returns an error when a response header cannot be represented as an HTTP
/// header name or value.
pub fn response_to_alb(response: &Response) -> AlbAdapterResult<AlbTargetGroupResponse> {
    let (body, is_base64_encoded) = alb_body(response.body());

    let mut alb_response = AlbTargetGroupResponse::default();
    alb_response.status_code = i64::from(response.status_code());
    alb_response.headers = response_headers(response.headers())?;
    alb_response.multi_value_headers = HeaderMap::new();
    alb_response.body = body;
    alb_response.is_base64_encoded = is_base64_encoded;

    Ok(alb_response)
}

impl Router {
    /// Handles an ALB target group event and returns an ALB target group response.
    ///
    /// # Errors
    ///
    /// Returns an error when request adapter conversion fails, except
    /// unsupported HTTP methods which are returned as `405 Method Not Allowed`.
    /// Also returns an error when response adapter conversion fails.
    pub fn handle_alb(
        &self,
        event: &AlbTargetGroupRequest,
    ) -> AlbAdapterResult<AlbTargetGroupResponse> {
        let request = match request_from_alb(event) {
            Ok(request) => request,
            Err(AlbAdapterError::UnsupportedMethod { .. }) => {
                return response_to_alb(&Response::new(405));
            }
            Err(error) => return Err(error),
        };
        response_to_alb(&self.handle(request))
    }
}

impl AsyncRouter {
    /// Handles an ALB target group event asynchronously.
    ///
    /// # Errors
    ///
    /// Returns an adapter error when request conversion fails, except
    /// unsupported HTTP methods which are returned as `405 Method Not Allowed`.
    /// Also returns an adapter error when response conversion fails.
    pub async fn handle_alb(
        &self,
        event: &AlbTargetGroupRequest,
    ) -> AlbAdapterResult<AlbTargetGroupResponse> {
        let request = match request_from_alb(event) {
            Ok(request) => request,
            Err(AlbAdapterError::UnsupportedMethod { .. }) => {
                return response_to_alb(&Response::new(405));
            }
            Err(error) => return Err(error),
        };
        response_to_alb(&self.handle(request).await)
    }
}

fn method_from_token(method: &str) -> AlbAdapterResult<Method> {
    Method::from_str(method).map_err(|_| AlbAdapterError::UnsupportedMethod {
        method: method.to_owned(),
    })
}

fn body_bytes(body: Option<&str>, is_base64_encoded: bool) -> AlbAdapterResult<Vec<u8>> {
    let Some(body) = body else {
        return Ok(Vec::new());
    };

    if is_base64_encoded {
        base64::engine::general_purpose::STANDARD
            .decode(body)
            .map_err(|error| AlbAdapterError::InvalidBodyEncoding {
                message: error.to_string(),
            })
    } else {
        Ok(body.as_bytes().to_vec())
    }
}

fn add_headers(mut request: Request, headers: &HeaderMap) -> AlbAdapterResult<Request> {
    for (name, value) in headers {
        let value = value
            .to_str()
            .map_err(|_| AlbAdapterError::InvalidRequestHeaderValue {
                name: name.as_str().to_owned(),
            })?;
        request = request.with_header(name.as_str(), value);
    }

    Ok(request)
}

fn response_headers(headers: &[(String, String)]) -> AlbAdapterResult<HeaderMap> {
    let mut output = HeaderMap::new();
    for (name, value) in headers {
        let header_name = HeaderName::from_str(name).map_err(|error| {
            AlbAdapterError::InvalidResponseHeaderName {
                name: name.clone(),
                message: error.to_string(),
            }
        })?;
        let header_value = HeaderValue::from_str(value).map_err(|error| {
            AlbAdapterError::InvalidResponseHeaderValue {
                name: name.clone(),
                message: error.to_string(),
            }
        })?;
        output.append(header_name, header_value);
    }

    Ok(output)
}

fn alb_body(body: &[u8]) -> (Option<Body>, bool) {
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
        event::alb::{AlbTargetGroupRequest, AlbTargetGroupResponse},
    };
    use http::Method as HttpMethod;

    use crate::{AlbAdapterError, Method, Response, Router, request_from_alb, response_to_alb};

    #[test]
    fn converts_alb_request() {
        let mut event = AlbTargetGroupRequest::default();
        event.path = Some("/orders/123".to_owned());
        event.http_method = HttpMethod::POST;
        event.body = Some("aGVsbG8=".to_owned());
        event.is_base64_encoded = true;
        event
            .headers
            .insert("x-request-id", "req-1".parse().unwrap());
        event.query_string_parameters =
            std::collections::HashMap::from([("debug".to_owned(), "true".to_owned())]).into();

        let request = request_from_alb(&event).expect("request converts");

        assert_eq!(request.method(), Method::Post);
        assert_eq!(request.path(), "/orders/123");
        assert_eq!(request.header("x-request-id"), Some("req-1"));
        assert_eq!(request.query_string_parameter("debug"), Some("true"));
        assert_eq!(request.body(), b"hello");
    }

    #[test]
    fn rejects_invalid_base64_body() {
        let mut event = AlbTargetGroupRequest::default();
        event.path = Some("/orders".to_owned());
        event.http_method = HttpMethod::POST;
        event.body = Some("not-base64!".to_owned());
        event.is_base64_encoded = true;

        assert!(matches!(
            request_from_alb(&event),
            Err(AlbAdapterError::InvalidBodyEncoding { .. })
        ));
    }

    #[test]
    fn converts_response_to_alb() {
        let response = Response::ok("created").with_header("content-type", "text/plain");

        let alb_response = response_to_alb(&response).expect("response converts");

        assert_eq!(alb_response.status_code, 200);
        assert_eq!(
            alb_response.headers.get("content-type").unwrap(),
            "text/plain"
        );
        assert_eq!(alb_response.body, Some(Body::Text("created".to_owned())));
        assert!(!alb_response.is_base64_encoded);
    }

    #[test]
    fn converts_binary_response_to_base64_alb_body() {
        let response = Response::new(200).with_body([0xff, 0x00]);

        let alb_response = response_to_alb(&response).expect("response converts");

        assert_eq!(alb_response.body, Some(Body::Binary(vec![0xff, 0x00])));
        assert!(alb_response.is_base64_encoded);
    }

    #[test]
    fn router_handles_alb_events() {
        let mut router = Router::new();
        router.get("/orders/{order_id}", |request| {
            Response::ok(request.path_param("order_id").unwrap_or_default())
        });
        let mut event = AlbTargetGroupRequest::default();
        event.path = Some("/orders/123".to_owned());
        event.http_method = HttpMethod::GET;

        let response: AlbTargetGroupResponse =
            router.handle_alb(&event).expect("router handles event");

        assert_eq!(response.status_code, 200);
        assert_eq!(response.body, Some(Body::Text("123".to_owned())));
    }

    #[test]
    fn router_returns_method_not_allowed_for_unsupported_alb_methods() {
        let router = Router::new();
        let mut event = AlbTargetGroupRequest::default();
        event.http_method = HttpMethod::TRACE;

        let response = router
            .handle_alb(&event)
            .expect("unsupported method returns a response");

        assert_eq!(response.status_code, 405);
        assert_eq!(response.body, None);
    }
}
