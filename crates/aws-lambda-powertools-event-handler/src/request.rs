//! HTTP request type.

use crate::{Extensions, Method, PathParams};

/// Dependency-free HTTP request passed to route handlers.
///
/// Equality compares HTTP fields and path parameters only; request-scoped and
/// shared extensions are execution context and are intentionally ignored.
#[derive(Clone, Debug)]
pub struct Request {
    method: Method,
    path: String,
    headers: Vec<(String, String)>,
    query_string_parameters: Vec<(String, String)>,
    path_params: PathParams,
    extensions: Extensions,
    shared_extensions: Extensions,
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
            extensions: Extensions::new(),
            shared_extensions: Extensions::new(),
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

    /// Returns request-scoped extension values.
    #[must_use]
    pub const fn extensions(&self) -> &Extensions {
        &self.extensions
    }

    /// Returns shared router extension values attached to this request.
    #[must_use]
    pub const fn shared_extensions(&self) -> &Extensions {
        &self.shared_extensions
    }

    /// Returns a request-scoped extension value by type.
    #[must_use]
    pub fn extension<T>(&self) -> Option<&T>
    where
        T: Send + Sync + 'static,
    {
        self.extensions.get::<T>()
    }

    /// Returns a shared router extension value by type.
    #[must_use]
    pub fn shared_extension<T>(&self) -> Option<&T>
    where
        T: Send + Sync + 'static,
    {
        self.shared_extensions.get::<T>()
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

    /// Adds or replaces a request-scoped extension value.
    #[must_use]
    pub fn with_extension<T>(mut self, value: T) -> Self
    where
        T: Send + Sync + 'static,
    {
        self.insert_extension(value);
        self
    }

    /// Adds or replaces a request-scoped extension value.
    pub fn insert_extension<T>(&mut self, value: T) -> &mut Self
    where
        T: Send + Sync + 'static,
    {
        self.extensions.insert(value);
        self
    }

    /// Removes all request-scoped extension values.
    pub fn clear_extensions(&mut self) -> &mut Self {
        self.extensions.clear();
        self
    }

    /// Adds a path parameter.
    #[must_use]
    pub fn with_path_param(mut self, name: impl AsRef<str>, value: impl AsRef<str>) -> Self {
        self.path_params.push(name.as_ref(), value.as_ref());
        self
    }

    pub(crate) fn set_path_params(&mut self, path_params: &PathParams) {
        let mut merged = self.path_params.clone();
        for (name, value) in path_params.iter() {
            if merged.get(name).is_none() {
                merged.push(name, value);
            }
        }
        self.path_params = merged;
    }

    pub(crate) fn set_shared_extensions(&mut self, shared_extensions: Extensions) {
        self.shared_extensions = shared_extensions;
    }
}

impl PartialEq for Request {
    fn eq(&self, other: &Self) -> bool {
        self.method == other.method
            && self.path == other.path
            && self.headers == other.headers
            && self.query_string_parameters == other.query_string_parameters
            && self.path_params == other.path_params
            && self.body == other.body
    }
}

impl Eq for Request {}
