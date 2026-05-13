//! Lambda context fields for structured logs.

/// Lambda context fields that can be appended to structured log entries.
pub trait LambdaLogContext {
    /// Returns the Lambda invocation request id.
    fn function_request_id(&self) -> &str;

    /// Returns the Lambda function name.
    fn function_name(&self) -> &str;

    /// Returns the Lambda function version, when available.
    fn function_version(&self) -> Option<&str> {
        None
    }

    /// Returns the Lambda function ARN, when available.
    fn function_arn(&self) -> Option<&str> {
        None
    }

    /// Returns the Lambda memory size in MB, when available.
    fn function_memory_size(&self) -> Option<u64> {
        None
    }

    /// Returns the cold-start flag, when available.
    fn cold_start(&self) -> Option<bool> {
        None
    }
}

/// Owned Lambda context fields for log enrichment.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LambdaContextFields {
    function_request_id: String,
    function_name: String,
    function_version: Option<String>,
    function_arn: Option<String>,
    function_memory_size: Option<u64>,
    cold_start: Option<bool>,
}

impl LambdaContextFields {
    /// Creates context fields with the required Lambda request id and function name.
    #[must_use]
    pub fn new(function_request_id: impl Into<String>, function_name: impl Into<String>) -> Self {
        Self {
            function_request_id: function_request_id.into(),
            function_name: function_name.into(),
            function_version: None,
            function_arn: None,
            function_memory_size: None,
            cold_start: None,
        }
    }

    /// Returns a copy with the Lambda function version.
    #[must_use]
    pub fn with_function_version(mut self, function_version: impl Into<String>) -> Self {
        self.function_version = Some(function_version.into());
        self
    }

    /// Returns a copy with the Lambda function ARN.
    #[must_use]
    pub fn with_function_arn(mut self, function_arn: impl Into<String>) -> Self {
        self.function_arn = Some(function_arn.into());
        self
    }

    /// Returns a copy with the Lambda memory size in MB.
    #[must_use]
    pub fn with_function_memory_size(mut self, function_memory_size: u64) -> Self {
        self.function_memory_size = Some(function_memory_size);
        self
    }

    /// Returns a copy with the cold-start flag.
    #[must_use]
    pub fn with_cold_start(mut self, cold_start: bool) -> Self {
        self.cold_start = Some(cold_start);
        self
    }
}

impl LambdaLogContext for LambdaContextFields {
    fn function_request_id(&self) -> &str {
        &self.function_request_id
    }

    fn function_name(&self) -> &str {
        &self.function_name
    }

    fn function_version(&self) -> Option<&str> {
        self.function_version.as_deref()
    }

    fn function_arn(&self) -> Option<&str> {
        self.function_arn.as_deref()
    }

    fn function_memory_size(&self) -> Option<u64> {
        self.function_memory_size
    }

    fn cold_start(&self) -> Option<bool> {
        self.cold_start
    }
}
