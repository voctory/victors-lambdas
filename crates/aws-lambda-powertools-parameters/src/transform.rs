//! Parameter transform errors.

/// High-level parameter transform error category.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParameterTransformErrorKind {
    /// A JSON value could not be deserialized into the requested type.
    Json,
    /// A binary value could not be decoded from base64.
    Binary,
}

/// Error returned when a parameter transform fails.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParameterTransformError {
    kind: ParameterTransformErrorKind,
    name: String,
    message: String,
}

impl ParameterTransformError {
    /// Creates an error for a JSON transform failure.
    #[must_use]
    pub fn json(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            kind: ParameterTransformErrorKind::Json,
            name: name.into(),
            message: message.into(),
        }
    }

    /// Creates an error for a binary transform failure.
    #[must_use]
    pub fn binary(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            kind: ParameterTransformErrorKind::Binary,
            name: name.into(),
            message: message.into(),
        }
    }

    /// Returns the transform error category.
    #[must_use]
    pub const fn kind(&self) -> ParameterTransformErrorKind {
        self.kind
    }

    /// Returns the parameter name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the transform error message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl std::fmt::Display for ParameterTransformError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{} transform failed: {}",
            self.name, self.message
        )
    }
}

impl std::error::Error for ParameterTransformError {}
