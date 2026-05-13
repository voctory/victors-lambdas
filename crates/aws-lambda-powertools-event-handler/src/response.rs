//! HTTP response type.

/// Dependency-free HTTP response returned from route handlers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Response {
    status_code: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl Response {
    /// Creates an empty response with the provided status code.
    #[must_use]
    pub const fn new(status_code: u16) -> Self {
        Self {
            status_code,
            headers: Vec::new(),
            body: Vec::new(),
        }
    }

    /// Creates a `200 OK` response with a body.
    #[must_use]
    pub fn ok(body: impl Into<Vec<u8>>) -> Self {
        Self::new(200).with_body(body)
    }

    /// Creates a `404 Not Found` response.
    #[must_use]
    pub fn not_found() -> Self {
        Self::new(404).with_body("Not Found")
    }

    /// Returns the response status code.
    #[must_use]
    pub const fn status_code(&self) -> u16 {
        self.status_code
    }

    /// Returns response headers in insertion order.
    #[must_use]
    pub fn headers(&self) -> &[(String, String)] {
        &self.headers
    }

    /// Returns a response header by case-insensitive name.
    #[must_use]
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.iter().find_map(|(header_name, value)| {
            header_name
                .eq_ignore_ascii_case(name)
                .then_some(value.as_str())
        })
    }

    /// Returns the response body bytes.
    #[must_use]
    pub fn body(&self) -> &[u8] {
        &self.body
    }

    /// Consumes the response and returns the body bytes.
    #[must_use]
    pub fn into_body(self) -> Vec<u8> {
        self.body
    }

    /// Adds a response header.
    #[must_use]
    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    /// Removes response headers that match a case-insensitive name.
    #[must_use]
    pub fn without_header(mut self, name: &str) -> Self {
        self.headers
            .retain(|(header_name, _)| !header_name.eq_ignore_ascii_case(name));
        self
    }

    /// Replaces all response headers matching `name` with one new value.
    #[must_use]
    pub fn with_replaced_header(self, name: impl Into<String>, value: impl Into<String>) -> Self {
        let name = name.into();
        self.without_header(&name).with_header(name, value)
    }

    /// Sets the response body bytes.
    #[must_use]
    pub fn with_body(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.body = body.into();
        self
    }
}
