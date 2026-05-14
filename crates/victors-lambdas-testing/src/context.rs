//! Lambda context test doubles.

/// Minimal Lambda context test double.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LambdaContextStub {
    request_id: String,
    function_name: String,
}

impl LambdaContextStub {
    /// Creates a Lambda context test double.
    #[must_use]
    pub fn new(request_id: impl Into<String>, function_name: impl Into<String>) -> Self {
        Self {
            request_id: request_id.into(),
            function_name: function_name.into(),
        }
    }

    /// Returns the request id.
    #[must_use]
    pub fn request_id(&self) -> &str {
        &self.request_id
    }

    /// Returns the function name.
    #[must_use]
    pub fn function_name(&self) -> &str {
        &self.function_name
    }
}

impl Default for LambdaContextStub {
    fn default() -> Self {
        Self::new("test-request-id", "test-function")
    }
}
