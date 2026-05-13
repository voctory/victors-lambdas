//! HTTP errors for fallible route handlers.

use std::fmt;

use crate::Response;

/// Error returned by fallible route handlers to produce an HTTP error response.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HttpError {
    status_code: u16,
    message: String,
}

impl HttpError {
    /// Creates an HTTP error with a status code and message.
    #[must_use]
    pub fn new(status_code: u16, message: impl Into<String>) -> Self {
        Self {
            status_code,
            message: message.into(),
        }
    }

    /// Creates a `400 Bad Request` error.
    #[must_use]
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(400, message)
    }

    /// Creates a `401 Unauthorized` error.
    #[must_use]
    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::new(401, message)
    }

    /// Creates a `403 Forbidden` error.
    #[must_use]
    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::new(403, message)
    }

    /// Creates a `404 Not Found` error.
    #[must_use]
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(404, message)
    }

    /// Creates a `405 Method Not Allowed` error.
    #[must_use]
    pub fn method_not_allowed(message: impl Into<String>) -> Self {
        Self::new(405, message)
    }

    /// Creates a `408 Request Timeout` error.
    #[must_use]
    pub fn request_timeout(message: impl Into<String>) -> Self {
        Self::new(408, message)
    }

    /// Creates a `409 Conflict` error.
    #[must_use]
    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new(409, message)
    }

    /// Creates a `413 Payload Too Large` error.
    #[must_use]
    pub fn payload_too_large(message: impl Into<String>) -> Self {
        Self::new(413, message)
    }

    /// Creates a `413 Request Entity Too Large` error.
    ///
    /// This aliases [`HttpError::payload_too_large`] for compatibility with
    /// Powertools terminology used by Python and TypeScript.
    #[must_use]
    pub fn request_entity_too_large(message: impl Into<String>) -> Self {
        Self::payload_too_large(message)
    }

    /// Creates a `422 Unprocessable Entity` error.
    #[must_use]
    pub fn unprocessable_entity(message: impl Into<String>) -> Self {
        Self::new(422, message)
    }

    /// Creates a `500 Internal Server Error` error.
    #[must_use]
    pub fn internal_server_error(message: impl Into<String>) -> Self {
        Self::new(500, message)
    }

    /// Creates a `503 Service Unavailable` error.
    #[must_use]
    pub fn service_unavailable(message: impl Into<String>) -> Self {
        Self::new(503, message)
    }

    /// Returns the HTTP status code.
    #[must_use]
    pub const fn status_code(&self) -> u16 {
        self.status_code
    }

    /// Returns the response message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Converts this error into a plain text response.
    #[must_use]
    pub fn into_response(self) -> Response {
        Response::new(self.status_code).with_body(self.message)
    }

    /// Converts this error into a plain text response.
    #[must_use]
    pub fn to_response(&self) -> Response {
        Response::new(self.status_code).with_body(self.message.clone())
    }
}

impl fmt::Display for HttpError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for HttpError {}

#[cfg(test)]
mod tests {
    use super::HttpError;

    #[test]
    fn helper_constructors_set_status_codes() {
        let bad_request = HttpError::bad_request("invalid payload");
        let not_found = HttpError::not_found("missing order");
        let request_timeout = HttpError::request_timeout("slow request");
        let request_entity_too_large = HttpError::request_entity_too_large("too large");
        let payload_too_large = HttpError::payload_too_large("payload too large");
        let service_unavailable = HttpError::service_unavailable("try later");

        assert_eq!(bad_request.status_code(), 400);
        assert_eq!(bad_request.message(), "invalid payload");
        assert_eq!(not_found.status_code(), 404);
        assert_eq!(not_found.message(), "missing order");
        assert_eq!(request_timeout.status_code(), 408);
        assert_eq!(request_entity_too_large.status_code(), 413);
        assert_eq!(payload_too_large.status_code(), 413);
        assert_eq!(service_unavailable.status_code(), 503);
    }

    #[test]
    fn converts_to_plain_text_response() {
        let response = HttpError::conflict("duplicate").into_response();

        assert_eq!(response.status_code(), 409);
        assert_eq!(response.body(), b"duplicate");
    }
}
