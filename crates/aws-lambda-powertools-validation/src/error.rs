//! Validation errors.

/// High-level validation error category.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValidationErrorKind {
    /// A required value was missing or blank.
    Required,
    /// A value was shorter than the minimum allowed length.
    TooShort,
    /// A value was longer than the maximum allowed length.
    TooLong,
    /// A numeric value was outside the allowed range.
    OutOfRange,
    /// A value was present but invalid for the field.
    Invalid,
    /// A JSON value did not satisfy a JSON Schema document.
    Schema,
    /// A caller-provided validation error.
    Custom,
}

/// Error returned when validation fails.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidationError {
    kind: ValidationErrorKind,
    field: Option<String>,
    message: String,
}

impl ValidationError {
    /// Creates a custom validation error message.
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            kind: ValidationErrorKind::Custom,
            field: None,
            message: message.into(),
        }
    }

    /// Creates an error for a required field.
    #[must_use]
    pub fn required(field: impl Into<String>) -> Self {
        let field = field.into();

        Self::with_field(
            ValidationErrorKind::Required,
            field.clone(),
            format!("{field} is required"),
        )
    }

    /// Creates an error for a field below its minimum text length.
    #[must_use]
    pub fn too_short(field: impl Into<String>, minimum: usize, actual: usize) -> Self {
        let field = field.into();

        Self::with_field(
            ValidationErrorKind::TooShort,
            field.clone(),
            format!("{field} must be at least {minimum} characters, got {actual}"),
        )
    }

    /// Creates an error for a field above its maximum text length.
    #[must_use]
    pub fn too_long(field: impl Into<String>, maximum: usize, actual: usize) -> Self {
        let field = field.into();

        Self::with_field(
            ValidationErrorKind::TooLong,
            field.clone(),
            format!("{field} must be at most {maximum} characters, got {actual}"),
        )
    }

    /// Creates an error for a numeric field outside its allowed range.
    #[must_use]
    pub fn out_of_range(field: impl Into<String>, minimum: i64, maximum: i64, actual: i64) -> Self {
        let field = field.into();

        Self::with_field(
            ValidationErrorKind::OutOfRange,
            field.clone(),
            format!("{field} must be between {minimum} and {maximum}, got {actual}"),
        )
    }

    /// Creates an error for a field that failed a custom validation predicate.
    #[must_use]
    pub fn invalid(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self::with_field(ValidationErrorKind::Invalid, field.into(), message.into())
    }

    /// Creates an error for a JSON Schema validation failure.
    #[must_use]
    pub fn json_schema(message: impl Into<String>) -> Self {
        Self {
            kind: ValidationErrorKind::Schema,
            field: None,
            message: message.into(),
        }
    }

    fn with_field(kind: ValidationErrorKind, field: String, message: String) -> Self {
        Self {
            kind,
            field: Some(field),
            message,
        }
    }

    /// Returns the validation error category.
    #[must_use]
    pub const fn kind(&self) -> ValidationErrorKind {
        self.kind
    }

    /// Returns the validated field name when available.
    #[must_use]
    pub fn field(&self) -> Option<&str> {
        self.field.as_deref()
    }

    /// Returns the error message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ValidationError {}
