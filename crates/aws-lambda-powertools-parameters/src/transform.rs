//! Parameter transform errors.

use serde_json::Value;

/// Parameter value transform to apply after retrieval.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ParameterTransform {
    /// Return the parameter value as text.
    #[default]
    None,
    /// Deserialize the parameter value as JSON.
    Json,
    /// Decode the parameter value as standard base64 bytes.
    Binary,
    /// Infer the transform from the parameter name suffix.
    ///
    /// Names ending in `.json` use [`ParameterTransform::Json`], names ending
    /// in `.binary` use [`ParameterTransform::Binary`], and all other names
    /// use [`ParameterTransform::None`]. Matching is case-insensitive.
    Auto,
}

impl ParameterTransform {
    /// Resolves [`ParameterTransform::Auto`] for a parameter name.
    #[must_use]
    pub fn resolve_for_name(self, name: &str) -> Self {
        match self {
            Self::Auto => {
                let extension = name.rsplit_once('.').map(|(_, extension)| extension);

                if extension.is_some_and(|extension| extension.eq_ignore_ascii_case("json")) {
                    Self::Json
                } else if extension
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("binary"))
                {
                    Self::Binary
                } else {
                    Self::None
                }
            }
            transform => transform,
        }
    }
}

/// Parameter value after a transform is applied.
#[derive(Clone, Debug, PartialEq)]
pub enum ParameterValue {
    /// Untransformed text value.
    Text(String),
    /// JSON value decoded from the parameter.
    Json(Value),
    /// Binary bytes decoded from a base64 parameter.
    Binary(Vec<u8>),
}

impl ParameterValue {
    /// Returns the inner text value when this is [`ParameterValue::Text`].
    #[must_use]
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(value) => Some(value),
            Self::Json(_) | Self::Binary(_) => None,
        }
    }

    /// Returns the inner JSON value when this is [`ParameterValue::Json`].
    #[must_use]
    pub const fn as_json(&self) -> Option<&Value> {
        match self {
            Self::Json(value) => Some(value),
            Self::Text(_) | Self::Binary(_) => None,
        }
    }

    /// Returns the inner bytes when this is [`ParameterValue::Binary`].
    #[must_use]
    pub fn as_binary(&self) -> Option<&[u8]> {
        match self {
            Self::Binary(value) => Some(value),
            Self::Text(_) | Self::Json(_) => None,
        }
    }
}

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
