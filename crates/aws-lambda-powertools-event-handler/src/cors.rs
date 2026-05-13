//! CORS configuration for HTTP routing.

use crate::{Method, Request, Response};

const ALLOW_ORIGIN: &str = "Access-Control-Allow-Origin";
const ALLOW_METHODS: &str = "Access-Control-Allow-Methods";
const ALLOW_HEADERS: &str = "Access-Control-Allow-Headers";
const ALLOW_CREDENTIALS: &str = "Access-Control-Allow-Credentials";
const EXPOSE_HEADERS: &str = "Access-Control-Expose-Headers";
const MAX_AGE: &str = "Access-Control-Max-Age";
const REQUEST_METHOD: &str = "Access-Control-Request-Method";

/// Cross-origin resource sharing settings for routed HTTP responses.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CorsConfig {
    allow_origin: String,
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
            allow_methods: vec![
                Method::Get,
                Method::Head,
                Method::Post,
                Method::Put,
                Method::Patch,
                Method::Delete,
                Method::Options,
            ],
            allow_headers: Vec::new(),
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

    /// Adds CORS response headers.
    #[must_use]
    pub fn apply(&self, response: Response) -> Response {
        let mut response = response
            .with_header(ALLOW_ORIGIN, self.allow_origin.clone())
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
        if self.allow_credentials {
            response = response.with_header(ALLOW_CREDENTIALS, "true");
        }

        response
    }

    /// Creates a CORS preflight response.
    #[must_use]
    pub fn preflight_response(&self) -> Response {
        self.apply(Response::new(204))
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
}
