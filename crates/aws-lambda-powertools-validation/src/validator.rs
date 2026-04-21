//! Validator facade.

use crate::ValidationError;

/// Result type returned by validation routines.
pub type ValidationResult = Result<(), ValidationError>;

/// Trait for payloads that can validate themselves through a validator facade.
pub trait Validate {
    /// Validates the receiver.
    ///
    /// # Errors
    ///
    /// Returns a validation error when the receiver violates one of its rules.
    fn validate(&self, validator: &Validator) -> ValidationResult;
}

/// Validates decoded events and payloads.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Validator;

impl Validator {
    /// Creates a validator.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Validates a value that implements the validation trait.
    ///
    /// # Errors
    ///
    /// Returns a validation error when the value violates one of its rules.
    pub fn validate<T>(&self, value: &T) -> ValidationResult
    where
        T: Validate + ?Sized,
    {
        value.validate(self)
    }

    /// Validates that a string value is not empty.
    ///
    /// # Errors
    ///
    /// Returns an error when `value` is empty after trimming whitespace.
    pub fn required_text(&self, value: &str) -> ValidationResult {
        self.required_text_field("value", value)
    }

    /// Validates that a named string value is not empty.
    ///
    /// # Errors
    ///
    /// Returns an error when `value` is empty after trimming whitespace.
    pub fn required_text_field(&self, field: impl Into<String>, value: &str) -> ValidationResult {
        if value.trim().is_empty() {
            Err(ValidationError::required(field))
        } else {
            Ok(())
        }
    }

    /// Validates that a named text value has at least `minimum` characters.
    ///
    /// # Errors
    ///
    /// Returns an error when `value` has fewer than `minimum` Unicode scalar
    /// values.
    pub fn text_min_chars(
        &self,
        field: impl Into<String>,
        value: &str,
        minimum: usize,
    ) -> ValidationResult {
        let actual = value.chars().count();

        if actual < minimum {
            Err(ValidationError::too_short(field, minimum, actual))
        } else {
            Ok(())
        }
    }

    /// Validates that a named text value has at most `maximum` characters.
    ///
    /// # Errors
    ///
    /// Returns an error when `value` has more than `maximum` Unicode scalar
    /// values.
    pub fn text_max_chars(
        &self,
        field: impl Into<String>,
        value: &str,
        maximum: usize,
    ) -> ValidationResult {
        let actual = value.chars().count();

        if actual > maximum {
            Err(ValidationError::too_long(field, maximum, actual))
        } else {
            Ok(())
        }
    }

    /// Validates that a named numeric value is inside an inclusive range.
    ///
    /// # Errors
    ///
    /// Returns an error when the bounds are invalid or when `value` is outside
    /// the inclusive range.
    pub fn i64_in_range(
        &self,
        field: impl Into<String>,
        value: i64,
        minimum: i64,
        maximum: i64,
    ) -> ValidationResult {
        let field = field.into();

        if minimum > maximum {
            return Err(ValidationError::invalid(
                field,
                "minimum bound exceeds maximum bound",
            ));
        }

        if (minimum..=maximum).contains(&value) {
            Ok(())
        } else {
            Err(ValidationError::out_of_range(
                field, minimum, maximum, value,
            ))
        }
    }

    /// Validates that a named predicate is true.
    ///
    /// # Errors
    ///
    /// Returns an error with `message` when `condition` is false.
    pub fn ensure(
        &self,
        field: impl Into<String>,
        condition: bool,
        message: impl Into<String>,
    ) -> ValidationResult {
        if condition {
            Ok(())
        } else {
            Err(ValidationError::invalid(field, message))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Validate, Validator};
    use crate::ValidationErrorKind;

    struct CreateOrder {
        order_id: String,
        quantity: i64,
    }

    impl Validate for CreateOrder {
        fn validate(&self, validator: &Validator) -> super::ValidationResult {
            validator.required_text_field("order_id", &self.order_id)?;
            validator.text_min_chars("order_id", &self.order_id, 3)?;
            validator.i64_in_range("quantity", self.quantity, 1, 10)
        }
    }

    #[test]
    fn required_text_keeps_existing_message() {
        let error = Validator::new()
            .required_text("  ")
            .expect_err("blank text should fail");

        assert_eq!(error.kind(), ValidationErrorKind::Required);
        assert_eq!(error.field(), Some("value"));
        assert_eq!(error.message(), "value is required");
    }

    #[test]
    fn text_length_errors_include_field_and_bounds() {
        let error = Validator::new()
            .text_min_chars("order_id", "ab", 3)
            .expect_err("short text should fail");

        assert_eq!(error.kind(), ValidationErrorKind::TooShort);
        assert_eq!(error.field(), Some("order_id"));
        assert_eq!(
            error.message(),
            "order_id must be at least 3 characters, got 2"
        );
    }

    #[test]
    fn range_errors_include_field_and_bounds() {
        let error = Validator::new()
            .i64_in_range("quantity", 11, 1, 10)
            .expect_err("out of range value should fail");

        assert_eq!(error.kind(), ValidationErrorKind::OutOfRange);
        assert_eq!(error.field(), Some("quantity"));
        assert_eq!(error.message(), "quantity must be between 1 and 10, got 11");
    }

    #[test]
    fn facade_validates_trait_implementations() {
        let valid = CreateOrder {
            order_id: String::from("ord-1"),
            quantity: 3,
        };
        let invalid = CreateOrder {
            order_id: String::from("ord-1"),
            quantity: 0,
        };
        let validator = Validator::new();

        assert!(validator.validate(&valid).is_ok());

        let error = validator
            .validate(&invalid)
            .expect_err("invalid order should fail");

        assert_eq!(error.kind(), ValidationErrorKind::OutOfRange);
        assert_eq!(error.field(), Some("quantity"));
    }

    #[test]
    fn ensure_builds_invalid_error() {
        let error = Validator::new()
            .ensure("order_id", false, "order_id must be unique")
            .expect_err("false predicate should fail");

        assert_eq!(error.kind(), ValidationErrorKind::Invalid);
        assert_eq!(error.field(), Some("order_id"));
        assert_eq!(error.message(), "order_id must be unique");
    }
}
