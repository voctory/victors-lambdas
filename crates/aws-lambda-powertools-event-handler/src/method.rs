//! HTTP methods for event routing.

/// HTTP method used for route matching.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Method {
    /// GET request.
    Get,
    /// POST request.
    Post,
    /// PUT request.
    Put,
    /// PATCH request.
    Patch,
    /// DELETE request.
    Delete,
    /// Any supported method.
    Any,
}
