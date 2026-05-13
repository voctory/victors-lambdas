//! VPC Lattice event adapters.

use std::{error::Error, fmt, str::FromStr};

use aws_lambda_events::{
    encodings::Body,
    event::vpc_lattice::{VpcLatticeRequestV1, VpcLatticeRequestV2, VpcLatticeResponse},
};
use base64::Engine;
use http::{HeaderMap, HeaderName, HeaderValue, Method as HttpMethod};

use crate::{AsyncRouter, Method, Request, Response, Router};

/// Result returned by VPC Lattice adapter operations.
pub type VpcLatticeAdapterResult<T> = Result<T, VpcLatticeAdapterError>;

/// Error returned by VPC Lattice adapter operations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VpcLatticeAdapterError {
    /// The incoming event did not include an HTTP method.
    MissingMethod,
    /// The incoming HTTP method is not supported by this router.
    UnsupportedMethod {
        /// Method token received from VPC Lattice.
        method: String,
    },
    /// VPC Lattice marked the body as base64 encoded but decoding failed.
    InvalidBodyEncoding {
        /// Decoding error message.
        message: String,
    },
    /// A request header value was not valid UTF-8.
    InvalidRequestHeaderValue {
        /// Header name.
        name: String,
    },
    /// A response header name is not valid for VPC Lattice.
    InvalidResponseHeaderName {
        /// Header name.
        name: String,
        /// Header parsing error message.
        message: String,
    },
    /// A response header value is not valid for VPC Lattice.
    InvalidResponseHeaderValue {
        /// Header name.
        name: String,
        /// Header parsing error message.
        message: String,
    },
}

impl fmt::Display for VpcLatticeAdapterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingMethod => write!(formatter, "VPC Lattice event is missing method"),
            Self::UnsupportedMethod { method } => {
                write!(formatter, "unsupported VPC Lattice HTTP method: {method}")
            }
            Self::InvalidBodyEncoding { message } => {
                write!(formatter, "invalid VPC Lattice body encoding: {message}")
            }
            Self::InvalidRequestHeaderValue { name } => {
                write!(
                    formatter,
                    "VPC Lattice request header {name} is not valid UTF-8"
                )
            }
            Self::InvalidResponseHeaderName { name, message } => {
                write!(
                    formatter,
                    "invalid VPC Lattice response header name {name}: {message}"
                )
            }
            Self::InvalidResponseHeaderValue { name, message } => {
                write!(
                    formatter,
                    "invalid VPC Lattice response header value for {name}: {message}"
                )
            }
        }
    }
}

impl Error for VpcLatticeAdapterError {}

/// Converts a VPC Lattice v1 event into a router request.
///
/// # Errors
///
/// Returns an error when the method is missing or unsupported, a header is not
/// UTF-8, or a base64 body cannot be decoded.
pub fn request_from_vpc_lattice(event: &VpcLatticeRequestV1) -> VpcLatticeAdapterResult<Request> {
    let method = method_from_http(event.method.as_ref())?;
    let path = event.raw_path.as_deref().unwrap_or("/");
    let mut request = Request::new(method, path)
        .with_body(body_bytes(event.body.as_ref(), event.is_base64_encoded)?);

    request = add_headers(request, &event.headers)?;
    for (name, value) in event.query_string_parameters.iter() {
        request = request.with_query_string_parameter(name, value);
    }

    Ok(request)
}

/// Converts a VPC Lattice v2 event into a router request.
///
/// # Errors
///
/// Returns an error when the method is missing or unsupported, a header is not
/// UTF-8, or a base64 body cannot be decoded.
pub fn request_from_vpc_lattice_v2(
    event: &VpcLatticeRequestV2,
) -> VpcLatticeAdapterResult<Request> {
    let method = method_from_http(event.method.as_ref())?;
    let path = event.path.as_deref().unwrap_or("/");
    let mut request = Request::new(method, path).with_body(string_body_bytes(
        event.body.as_deref(),
        event.is_base64_encoded,
    )?);

    request = add_headers(request, &event.headers)?;
    for (name, value) in event.query_string_parameters.iter() {
        request = request.with_query_string_parameter(name, value);
    }

    Ok(request)
}

/// Converts a router response into a VPC Lattice response.
///
/// # Errors
///
/// Returns an error when a response header cannot be represented as an HTTP
/// header name or value.
pub fn response_to_vpc_lattice(response: &Response) -> VpcLatticeAdapterResult<VpcLatticeResponse> {
    let (body, is_base64_encoded) = lattice_body(response.body());

    let mut lattice_response = VpcLatticeResponse::default();
    lattice_response.status_code = response.status_code();
    lattice_response.headers = response_headers(response.headers())?;
    lattice_response.body = body;
    lattice_response.is_base64_encoded = is_base64_encoded;

    Ok(lattice_response)
}

impl Router {
    /// Handles a VPC Lattice v1 event and returns a VPC Lattice response.
    ///
    /// # Errors
    ///
    /// Returns an error when request adapter conversion fails, except
    /// unsupported HTTP methods which are returned as `405 Method Not Allowed`.
    /// Also returns an error when response adapter conversion fails.
    pub fn handle_vpc_lattice(
        &self,
        event: &VpcLatticeRequestV1,
    ) -> VpcLatticeAdapterResult<VpcLatticeResponse> {
        let request = match request_from_vpc_lattice(event) {
            Ok(request) => request,
            Err(VpcLatticeAdapterError::UnsupportedMethod { .. }) => {
                return response_to_vpc_lattice(&Response::new(405));
            }
            Err(error) => return Err(error),
        };
        response_to_vpc_lattice(&self.handle(request))
    }

    /// Handles a VPC Lattice v2 event and returns a VPC Lattice response.
    ///
    /// # Errors
    ///
    /// Returns an error when request adapter conversion fails, except
    /// unsupported HTTP methods which are returned as `405 Method Not Allowed`.
    /// Also returns an error when response adapter conversion fails.
    pub fn handle_vpc_lattice_v2(
        &self,
        event: &VpcLatticeRequestV2,
    ) -> VpcLatticeAdapterResult<VpcLatticeResponse> {
        let request = match request_from_vpc_lattice_v2(event) {
            Ok(request) => request,
            Err(VpcLatticeAdapterError::UnsupportedMethod { .. }) => {
                return response_to_vpc_lattice(&Response::new(405));
            }
            Err(error) => return Err(error),
        };
        response_to_vpc_lattice(&self.handle(request))
    }
}

impl AsyncRouter {
    /// Handles a VPC Lattice v1 event asynchronously.
    ///
    /// # Errors
    ///
    /// Returns an adapter error when request conversion fails, except
    /// unsupported HTTP methods which are returned as `405 Method Not Allowed`.
    /// Also returns an adapter error when response conversion fails.
    pub async fn handle_vpc_lattice(
        &self,
        event: &VpcLatticeRequestV1,
    ) -> VpcLatticeAdapterResult<VpcLatticeResponse> {
        let request = match request_from_vpc_lattice(event) {
            Ok(request) => request,
            Err(VpcLatticeAdapterError::UnsupportedMethod { .. }) => {
                return response_to_vpc_lattice(&Response::new(405));
            }
            Err(error) => return Err(error),
        };
        response_to_vpc_lattice(&self.handle(request).await)
    }

    /// Handles a VPC Lattice v2 event asynchronously.
    ///
    /// # Errors
    ///
    /// Returns an adapter error when request conversion fails, except
    /// unsupported HTTP methods which are returned as `405 Method Not Allowed`.
    /// Also returns an adapter error when response conversion fails.
    pub async fn handle_vpc_lattice_v2(
        &self,
        event: &VpcLatticeRequestV2,
    ) -> VpcLatticeAdapterResult<VpcLatticeResponse> {
        let request = match request_from_vpc_lattice_v2(event) {
            Ok(request) => request,
            Err(VpcLatticeAdapterError::UnsupportedMethod { .. }) => {
                return response_to_vpc_lattice(&Response::new(405));
            }
            Err(error) => return Err(error),
        };
        response_to_vpc_lattice(&self.handle(request).await)
    }
}

fn method_from_http(method: Option<&HttpMethod>) -> VpcLatticeAdapterResult<Method> {
    let method = method.ok_or(VpcLatticeAdapterError::MissingMethod)?;
    Method::from_str(method.as_str()).map_err(|_| VpcLatticeAdapterError::UnsupportedMethod {
        method: method.as_str().to_owned(),
    })
}

fn body_bytes(body: Option<&Body>, is_base64_encoded: bool) -> VpcLatticeAdapterResult<Vec<u8>> {
    let Some(body) = body else {
        return Ok(Vec::new());
    };

    if is_base64_encoded {
        let encoded = std::str::from_utf8(body.as_ref()).map_err(|error| {
            VpcLatticeAdapterError::InvalidBodyEncoding {
                message: error.to_string(),
            }
        })?;
        base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .map_err(|error| VpcLatticeAdapterError::InvalidBodyEncoding {
                message: error.to_string(),
            })
    } else {
        Ok(body.as_ref().to_vec())
    }
}

fn string_body_bytes(
    body: Option<&str>,
    is_base64_encoded: bool,
) -> VpcLatticeAdapterResult<Vec<u8>> {
    let Some(body) = body else {
        return Ok(Vec::new());
    };

    if is_base64_encoded {
        base64::engine::general_purpose::STANDARD
            .decode(body)
            .map_err(|error| VpcLatticeAdapterError::InvalidBodyEncoding {
                message: error.to_string(),
            })
    } else {
        Ok(body.as_bytes().to_vec())
    }
}

fn add_headers(mut request: Request, headers: &HeaderMap) -> VpcLatticeAdapterResult<Request> {
    for (name, value) in headers {
        let value =
            value
                .to_str()
                .map_err(|_| VpcLatticeAdapterError::InvalidRequestHeaderValue {
                    name: name.as_str().to_owned(),
                })?;
        request = request.with_header(name.as_str(), value);
    }

    Ok(request)
}

fn response_headers(headers: &[(String, String)]) -> VpcLatticeAdapterResult<HeaderMap> {
    let mut output = HeaderMap::new();
    for (name, value) in headers {
        let header_name = HeaderName::from_str(name).map_err(|error| {
            VpcLatticeAdapterError::InvalidResponseHeaderName {
                name: name.clone(),
                message: error.to_string(),
            }
        })?;
        let header_value = HeaderValue::from_str(value).map_err(|error| {
            VpcLatticeAdapterError::InvalidResponseHeaderValue {
                name: name.clone(),
                message: error.to_string(),
            }
        })?;
        output.append(header_name, header_value);
    }

    Ok(output)
}

fn lattice_body(body: &[u8]) -> (Option<Body>, bool) {
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
        event::vpc_lattice::{VpcLatticeRequestV1, VpcLatticeRequestV2},
    };
    use http::Method as HttpMethod;

    use crate::{
        Method, Response, Router, VpcLatticeAdapterError, request_from_vpc_lattice,
        request_from_vpc_lattice_v2, response_to_vpc_lattice,
    };

    #[test]
    fn converts_vpc_lattice_request() {
        let mut event = VpcLatticeRequestV1::default();
        event.raw_path = Some("/orders/123".to_owned());
        event.method = Some(HttpMethod::POST);
        event.body = Some(Body::from("aGVsbG8="));
        event.is_base64_encoded = true;
        event
            .headers
            .insert("x-request-id", "req-1".parse().unwrap());
        event.query_string_parameters =
            std::collections::HashMap::from([("debug".to_owned(), "true".to_owned())]).into();

        let request = request_from_vpc_lattice(&event).expect("request converts");

        assert_eq!(request.method(), Method::Post);
        assert_eq!(request.path(), "/orders/123");
        assert_eq!(request.header("x-request-id"), Some("req-1"));
        assert_eq!(request.query_string_parameter("debug"), Some("true"));
        assert_eq!(request.body(), b"hello");
    }

    #[test]
    fn converts_vpc_lattice_v2_request() {
        let mut event = VpcLatticeRequestV2::default();
        event.path = Some("/orders/123".to_owned());
        event.method = Some(HttpMethod::POST);
        event.body = Some("aGVsbG8=".to_owned());
        event.is_base64_encoded = true;
        event
            .headers
            .insert("x-request-id", "req-1".parse().unwrap());
        event.query_string_parameters =
            std::collections::HashMap::from([("debug".to_owned(), "true".to_owned())]).into();

        let request = request_from_vpc_lattice_v2(&event).expect("request converts");

        assert_eq!(request.method(), Method::Post);
        assert_eq!(request.path(), "/orders/123");
        assert_eq!(request.header("x-request-id"), Some("req-1"));
        assert_eq!(request.query_string_parameter("debug"), Some("true"));
        assert_eq!(request.body(), b"hello");
    }

    #[test]
    fn rejects_missing_method() {
        let event = VpcLatticeRequestV1::default();

        assert_eq!(
            request_from_vpc_lattice(&event),
            Err(VpcLatticeAdapterError::MissingMethod)
        );
    }

    #[test]
    fn rejects_invalid_base64_body() {
        let mut event = VpcLatticeRequestV2::default();
        event.path = Some("/orders".to_owned());
        event.method = Some(HttpMethod::POST);
        event.body = Some("not-base64!".to_owned());
        event.is_base64_encoded = true;

        assert!(matches!(
            request_from_vpc_lattice_v2(&event),
            Err(VpcLatticeAdapterError::InvalidBodyEncoding { .. })
        ));
    }

    #[test]
    fn converts_response_to_vpc_lattice() {
        let response = Response::ok("created").with_header("content-type", "text/plain");

        let lattice_response = response_to_vpc_lattice(&response).expect("response converts");

        assert_eq!(lattice_response.status_code, 200);
        assert_eq!(
            lattice_response.headers.get("content-type").unwrap(),
            "text/plain"
        );
        assert_eq!(
            lattice_response.body,
            Some(Body::Text("created".to_owned()))
        );
        assert!(!lattice_response.is_base64_encoded);
    }

    #[test]
    fn converts_binary_response_to_base64_vpc_lattice_body() {
        let response = Response::new(200).with_body([0xff, 0x00]);

        let lattice_response = response_to_vpc_lattice(&response).expect("response converts");

        assert_eq!(lattice_response.body, Some(Body::Binary(vec![0xff, 0x00])));
        assert!(lattice_response.is_base64_encoded);
    }

    #[test]
    fn router_handles_vpc_lattice_events() {
        let mut router = Router::new();
        router.get("/orders/{order_id}", |request| {
            Response::ok(request.path_param("order_id").unwrap_or_default())
        });
        let mut event = VpcLatticeRequestV1::default();
        event.raw_path = Some("/orders/123".to_owned());
        event.method = Some(HttpMethod::GET);

        let response = router
            .handle_vpc_lattice(&event)
            .expect("router handles event");

        assert_eq!(response.status_code, 200);
        assert_eq!(response.body, Some(Body::Text("123".to_owned())));
    }

    #[test]
    fn router_handles_vpc_lattice_v2_events() {
        let mut router = Router::new();
        router.get("/orders/{order_id}", |request| {
            Response::ok(request.path_param("order_id").unwrap_or_default())
        });
        let mut event = VpcLatticeRequestV2::default();
        event.path = Some("/orders/123".to_owned());
        event.method = Some(HttpMethod::GET);

        let response = router
            .handle_vpc_lattice_v2(&event)
            .expect("router handles event");

        assert_eq!(response.status_code, 200);
        assert_eq!(response.body, Some(Body::Text("123".to_owned())));
    }

    #[test]
    fn router_returns_method_not_allowed_for_unsupported_vpc_lattice_methods() {
        let router = Router::new();
        let mut v1_event = VpcLatticeRequestV1::default();
        v1_event.method = Some(HttpMethod::TRACE);
        let mut v2_event = VpcLatticeRequestV2::default();
        v2_event.method = Some(HttpMethod::CONNECT);

        let v1_response = router
            .handle_vpc_lattice(&v1_event)
            .expect("unsupported method returns a response");
        let v2_response = router
            .handle_vpc_lattice_v2(&v2_event)
            .expect("unsupported method returns a response");

        assert_eq!(v1_response.status_code, 405);
        assert_eq!(v1_response.body, None);
        assert_eq!(v2_response.status_code, 405);
        assert_eq!(v2_response.body, None);
    }
}
