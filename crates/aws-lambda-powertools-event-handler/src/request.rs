//! HTTP request type.

use crate::{Method, PathParams};

/// Dependency-free HTTP request passed to route handlers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Request {
    method: Method,
    path: String,
    headers: Vec<(String, String)>,
    query_string_parameters: Vec<(String, String)>,
    path_params: PathParams,
    body: Vec<u8>,
}

impl Request {
    /// Creates a request with an HTTP method and path.
    #[must_use]
    pub fn new(method: Method, path: impl Into<String>) -> Self {
        Self {
            method,
            path: path.into(),
            headers: Vec::new(),
            query_string_parameters: Vec::new(),
            path_params: PathParams::new(),
            body: Vec::new(),
        }
    }

    /// Returns the request method.
    #[must_use]
    pub fn method(&self) -> Method {
        self.method
    }

    /// Returns the request path.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Returns request headers in insertion order.
    #[must_use]
    pub fn headers(&self) -> &[(String, String)] {
        &self.headers
    }

    /// Returns a request header by case-insensitive name.
    #[must_use]
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.iter().find_map(|(header_name, value)| {
            header_name
                .eq_ignore_ascii_case(name)
                .then_some(value.as_str())
        })
    }

    /// Returns query string parameters in insertion order.
    #[must_use]
    pub fn query_string_parameters(&self) -> &[(String, String)] {
        &self.query_string_parameters
    }

    /// Returns a query string parameter by exact name.
    #[must_use]
    pub fn query_string_parameter(&self, name: &str) -> Option<&str> {
        self.query_string_parameters
            .iter()
            .find_map(|(parameter_name, value)| (parameter_name == name).then_some(value.as_str()))
    }

    /// Returns path parameters captured by the matched route.
    #[must_use]
    pub fn path_params(&self) -> &PathParams {
        &self.path_params
    }

    /// Returns a captured path parameter by name.
    #[must_use]
    pub fn path_param(&self, name: &str) -> Option<&str> {
        self.path_params.get(name)
    }

    /// Returns the request body bytes.
    #[must_use]
    pub fn body(&self) -> &[u8] {
        &self.body
    }

    /// Adds a request header.
    #[must_use]
    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    /// Adds a query string parameter.
    #[must_use]
    pub fn with_query_string_parameter(
        mut self,
        name: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        self.query_string_parameters
            .push((name.into(), value.into()));
        self
    }

    /// Sets the request body bytes.
    #[must_use]
    pub fn with_body(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.body = body.into();
        self
    }

    /// Adds a path parameter.
    #[must_use]
    pub fn with_path_param(mut self, name: impl AsRef<str>, value: impl AsRef<str>) -> Self {
        self.path_params.push(name.as_ref(), value.as_ref());
        self
    }

    pub(crate) fn set_path_params(&mut self, path_params: PathParams) {
        self.path_params = path_params;
    }
}
