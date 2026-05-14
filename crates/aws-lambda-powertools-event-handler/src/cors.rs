//! CORS configuration for HTTP routing.

use crate::{Method, Request, Response};

const ALLOW_ORIGIN: &str = "Access-Control-Allow-Origin";
const ALLOW_METHODS: &str = "Access-Control-Allow-Methods";
const ALLOW_HEADERS: &str = "Access-Control-Allow-Headers";
const ALLOW_CREDENTIALS: &str = "Access-Control-Allow-Credentials";
const EXPOSE_HEADERS: &str = "Access-Control-Expose-Headers";
const MAX_AGE: &str = "Access-Control-Max-Age";
const REQUEST_METHOD: &str = "Access-Control-Request-Method";
const REQUEST_HEADERS: &str = "Access-Control-Request-Headers";

const DEFAULT_ALLOW_HEADERS: [&str; 5] = [
    "Authorization",
    "Content-Type",
    "X-Amz-Date",
    "X-Api-Key",
    "X-Amz-Security-Token",
];

/// Cross-origin resource sharing settings for routed HTTP responses.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CorsConfig {
    allow_origin: String,
    extra_origins: Vec<String>,
    allow_methods: Vec<Method>,
    allow_headers: Vec<String>,
    expose_headers: Vec<String>,
    max_age_seconds: Option<u64>,
    allow_credentials: bool,
}

impl CorsConfig {
    /// Creates a CORS configuration with an allowed origin.
    #[must_use]
    pub fn new(allow_origin: impl Into<String>) -> Self {
        Self {
            allow_origin: allow_origin.into(),
            extra_origins: Vec::new(),
            allow_methods: vec![
                Method::Get,
                Method::Head,
                Method::Post,
                Method::Put,
                Method::Patch,
                Method::Delete,
                Method::Options,
            ],
            allow_headers: DEFAULT_ALLOW_HEADERS
                .iter()
                .map(ToString::to_string)
                .collect(),
            expose_headers: Vec::new(),
            max_age_seconds: None,
            allow_credentials: false,
        }
    }

    /// Returns the allowed origin value.
    #[must_use]
    pub fn allow_origin(&self) -> &str {
        &self.allow_origin
    }

    /// Returns additional allowed origins.
    #[must_use]
    pub fn extra_origins(&self) -> &[String] {
        &self.extra_origins
    }

    /// Returns allowed methods.
    #[must_use]
    pub fn allow_methods(&self) -> &[Method] {
        &self.allow_methods
    }

    /// Returns allowed request headers.
    #[must_use]
    pub fn allow_headers(&self) -> &[String] {
        &self.allow_headers
    }

    /// Returns exposed response headers.
    #[must_use]
    pub fn expose_headers(&self) -> &[String] {
        &self.expose_headers
    }

    /// Returns the preflight max age in seconds.
    #[must_use]
    pub const fn max_age_seconds(&self) -> Option<u64> {
        self.max_age_seconds
    }

    /// Returns whether credentials are allowed.
    #[must_use]
    pub const fn allow_credentials(&self) -> bool {
        self.allow_credentials
    }

    /// Returns a copy with additional allowed origins replaced.
    #[must_use]
    pub fn with_extra_origins(
        mut self,
        origins: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.extra_origins = origins.into_iter().map(Into::into).collect();
        self
    }

    /// Returns a copy with allowed methods replaced.
    #[must_use]
    pub fn with_allow_methods(mut self, methods: impl IntoIterator<Item = Method>) -> Self {
        self.allow_methods = methods.into_iter().collect();
        self
    }

    /// Returns a copy with allowed request headers replaced.
    #[must_use]
    pub fn with_allow_headers(
        mut self,
        headers: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.allow_headers = headers.into_iter().map(Into::into).collect();
        self
    }

    /// Returns a copy with exposed response headers replaced.
    #[must_use]
    pub fn with_expose_headers(
        mut self,
        headers: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.expose_headers = headers.into_iter().map(Into::into).collect();
        self
    }

    /// Returns a copy with a preflight max age in seconds.
    #[must_use]
    pub const fn with_max_age_seconds(mut self, max_age_seconds: u64) -> Self {
        self.max_age_seconds = Some(max_age_seconds);
        self
    }

    /// Returns a copy with credential support enabled or disabled.
    ///
    /// Credential headers are emitted only when the matched origin is not `*`.
    #[must_use]
    pub const fn with_allow_credentials(mut self, allow_credentials: bool) -> Self {
        self.allow_credentials = allow_credentials;
        self
    }

    /// Returns true when the request is a CORS preflight request.
    #[must_use]
    pub fn is_preflight_request(&self, request: &Request) -> bool {
        request.method() == Method::Options && request.header(REQUEST_METHOD).is_some()
    }

    /// Adds CORS response headers when the request origin is allowed.
    #[must_use]
    pub fn apply_for_request(&self, request: &Request, response: Response) -> Response {
        let Some(origin) = self.origin_for_request(request) else {
            return response;
        };

        self.apply_with_origin(response, origin)
    }

    /// Adds CORS response headers.
    #[must_use]
    pub fn apply(&self, response: Response) -> Response {
        self.apply_with_origin(response, self.allow_origin.as_str())
    }

    /// Creates a CORS preflight response when the request origin and method are allowed.
    #[must_use]
    pub fn preflight_response_for_request(&self, request: &Request) -> Option<Response> {
        if !self.is_preflight_request(request) || self.origin_for_request(request).is_none() {
            return None;
        }
        if !self.allows_requested_method(request) || !self.allows_requested_headers(request) {
            return None;
        }

        Some(self.apply_for_request(request, Response::new(204)))
    }

    /// Creates a CORS preflight response.
    #[must_use]
    pub fn preflight_response(&self) -> Response {
        self.apply(Response::new(204))
    }

    fn apply_with_origin(&self, response: Response, origin: &str) -> Response {
        let mut response = response
            .with_header(ALLOW_ORIGIN, origin)
            .with_header(ALLOW_METHODS, join_methods(&self.allow_methods));

        if !self.allow_headers.is_empty() {
            response = response.with_header(ALLOW_HEADERS, self.allow_headers.join(","));
        }
        if !self.expose_headers.is_empty() {
            response = response.with_header(EXPOSE_HEADERS, self.expose_headers.join(","));
        }
        if let Some(max_age_seconds) = self.max_age_seconds {
            response = response.with_header(MAX_AGE, max_age_seconds.to_string());
        }
        if self.allow_credentials && origin != "*" {
            response = response.with_header(ALLOW_CREDENTIALS, "true");
        }

        response
    }

    fn origin_for_request<'a>(&self, request: &'a Request) -> Option<&'a str> {
        let origin = request.header("Origin")?;

        if self.allows_wildcard_origin() {
            return Some("*");
        }

        if self.allow_origin == origin || self.extra_origins.iter().any(|extra| extra == origin) {
            Some(origin)
        } else {
            None
        }
    }

    fn allows_wildcard_origin(&self) -> bool {
        self.allow_origin == "*" || self.extra_origins.iter().any(|origin| origin == "*")
    }

    fn allows_requested_method(&self, request: &Request) -> bool {
        request
            .header(REQUEST_METHOD)
            .is_some_and(|requested_method| {
                self.allow_methods
                    .iter()
                    .any(|method| method.as_str().eq_ignore_ascii_case(requested_method))
            })
    }

    fn allows_requested_headers(&self, request: &Request) -> bool {
        request
            .header(REQUEST_HEADERS)
            .is_none_or(|requested_headers| {
                requested_headers.split(',').all(|requested_header| {
                    let requested_header = requested_header.trim();
                    self.allow_headers
                        .iter()
                        .any(|allowed_header| allowed_header.eq_ignore_ascii_case(requested_header))
                })
            })
    }
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self::new("*")
    }
}

fn join_methods(methods: &[Method]) -> String {
    methods
        .iter()
        .map(|method| method.as_str())
        .collect::<Vec<_>>()
        .join(",")
}

#[cfg(test)]
mod tests {
    use crate::{CorsConfig, Method, Request, Response};

    #[test]
    fn applies_cors_headers_to_response() {
        let cors = CorsConfig::new("https://example.com")
            .with_extra_origins(["https://app.example.com"])
            .with_allow_methods([Method::Get, Method::Post])
            .with_allow_headers(["authorization", "content-type"])
            .with_expose_headers(["x-request-id"])
            .with_max_age_seconds(600)
            .with_allow_credentials(true);

        let response = cors.apply(Response::ok("ok"));

        assert_eq!(
            response.header("access-control-allow-origin"),
            Some("https://example.com")
        );
        assert_eq!(
            response.header("access-control-allow-methods"),
            Some("GET,POST")
        );
        assert_eq!(
            response.header("access-control-allow-headers"),
            Some("authorization,content-type")
        );
        assert_eq!(
            response.header("access-control-expose-headers"),
            Some("x-request-id")
        );
        assert_eq!(response.header("access-control-max-age"), Some("600"));
        assert_eq!(
            response.header("access-control-allow-credentials"),
            Some("true")
        );
        assert_eq!(cors.extra_origins(), &["https://app.example.com"]);
    }

    #[test]
    fn default_cors_allows_aws_request_headers() {
        let response = CorsConfig::default().apply(Response::ok("ok"));

        assert_eq!(
            response.header("access-control-allow-headers"),
            Some("Authorization,Content-Type,X-Amz-Date,X-Api-Key,X-Amz-Security-Token")
        );
    }

    #[test]
    fn request_aware_cors_uses_matching_origin() {
        let cors =
            CorsConfig::new("https://example.com").with_extra_origins(["https://app.example.com"]);
        let request =
            Request::new(Method::Get, "/orders").with_header("Origin", "https://app.example.com");

        let response = cors.apply_for_request(&request, Response::ok("ok"));

        assert_eq!(
            response.header("access-control-allow-origin"),
            Some("https://app.example.com")
        );
    }

    #[test]
    fn wildcard_origin_does_not_emit_credentials_header() {
        let cors = CorsConfig::default().with_allow_credentials(true);
        let request =
            Request::new(Method::Get, "/orders").with_header("Origin", "https://app.example.com");

        let response = cors.apply_for_request(&request, Response::ok("ok"));

        assert_eq!(response.header("access-control-allow-origin"), Some("*"));
        assert_eq!(response.header("access-control-allow-credentials"), None);
    }

    #[test]
    fn request_aware_cors_ignores_disallowed_origin() {
        let cors = CorsConfig::new("https://example.com");
        let request =
            Request::new(Method::Get, "/orders").with_header("Origin", "https://app.example.com");

        let response = cors.apply_for_request(&request, Response::ok("ok"));

        assert_eq!(response.header("access-control-allow-origin"), None);
    }

    #[test]
    fn identifies_preflight_requests() {
        let cors = CorsConfig::default();
        let request = Request::new(Method::Options, "/orders")
            .with_header("Access-Control-Request-Method", "POST");

        assert!(cors.is_preflight_request(&request));
        assert!(!cors.is_preflight_request(&Request::new(Method::Options, "/orders")));
    }

    #[test]
    fn preflight_response_uses_no_content_status() {
        let response = CorsConfig::default().preflight_response();

        assert_eq!(response.status_code(), 204);
        assert_eq!(response.header("access-control-allow-origin"), Some("*"));
    }

    #[test]
    fn preflight_response_for_request_validates_origin_method_and_headers() {
        let cors = CorsConfig::new("https://example.com").with_allow_methods([Method::Post]);
        let request = Request::new(Method::Options, "/orders")
            .with_header("Origin", "https://example.com")
            .with_header("Access-Control-Request-Method", "POST")
            .with_header("Access-Control-Request-Headers", "content-type,x-amz-date");

        let response = cors
            .preflight_response_for_request(&request)
            .expect("preflight should be allowed");

        assert_eq!(response.status_code(), 204);
        assert_eq!(
            response.header("access-control-allow-origin"),
            Some("https://example.com")
        );

        let disallowed = Request::new(Method::Options, "/orders")
            .with_header("Origin", "https://evil.example.com")
            .with_header("Access-Control-Request-Method", "POST");
        assert!(cors.preflight_response_for_request(&disallowed).is_none());
    }
}
