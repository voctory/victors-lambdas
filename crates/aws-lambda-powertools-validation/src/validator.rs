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

    /// Validates an inbound value and returns it unchanged when it passes.
    ///
    /// This is useful in handler pipelines that want to validate decoded input
    /// before handing ownership to business logic.
    ///
    /// # Errors
    ///
    /// Returns a validation error when the value violates one of its rules.
    pub fn validate_inbound<T>(&self, value: T) -> Result<T, ValidationError>
    where
        T: Validate,
    {
        self.validate(&value)?;
        Ok(value)
    }

    /// Validates an outbound value and returns it unchanged when it passes.
    ///
    /// This is useful in handler pipelines that want to validate responses
    /// before serializing them.
    ///
    /// # Errors
    ///
    /// Returns a validation error when the value violates one of its rules.
    pub fn validate_outbound<T>(&self, value: T) -> Result<T, ValidationError>
    where
        T: Validate,
    {
        self.validate(&value)?;
        Ok(value)
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

    /// Validates a JSON value against a JSON Schema document.
    ///
    /// This method is available with the `jsonschema` feature and validates
    /// only in-memory schemas; remote reference resolution is intentionally not
    /// enabled by default.
    ///
    /// # Errors
    ///
    /// Returns a validation error when the schema is invalid or when `instance`
    /// does not satisfy the schema.
    #[cfg(feature = "jsonschema")]
    pub fn json_schema(
        &self,
        schema: &serde_json::Value,
        instance: &serde_json::Value,
    ) -> ValidationResult {
        let validator = jsonschema::validator_for(schema)
            .map_err(|error| ValidationError::invalid("schema", error.to_string()))?;

        validator
            .validate(instance)
            .map_err(|error| ValidationError::json_schema(error.to_string()))
    }

    /// Validates a JSON value selected by a `JMESPath` envelope against a JSON Schema document.
    ///
    /// This method is available with the `jmespath` feature, which also enables
    /// JSON Schema validation. Powertools `JMESPath` helper functions such as
    /// `powertools_json`, `powertools_base64`, and `powertools_base64_gzip` are
    /// available in the envelope expression.
    ///
    /// # Errors
    ///
    /// Returns a validation error when the envelope cannot be extracted, when
    /// the schema is invalid, or when the selected instance does not satisfy
    /// the schema.
    #[cfg(feature = "jmespath")]
    pub fn json_schema_envelope(
        &self,
        schema: &serde_json::Value,
        event: &serde_json::Value,
        envelope: &str,
    ) -> ValidationResult {
        let instance = crate::extract_envelope(event, envelope)?;

        self.json_schema(schema, &instance)
    }
}

#[cfg(test)]
mod tests {
    use super::{Validate, Validator};
    use crate::ValidationErrorKind;

    #[derive(Debug)]
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
    fn validate_inbound_returns_valid_value() {
        let order = CreateOrder {
            order_id: String::from("ord-1"),
            quantity: 3,
        };

        let order = Validator::new()
            .validate_inbound(order)
            .expect("valid inbound value should pass");

        assert_eq!(order.order_id, "ord-1");
        assert_eq!(order.quantity, 3);
    }

    #[test]
    fn validate_outbound_returns_validation_errors() {
        let order = CreateOrder {
            order_id: String::from("ord-1"),
            quantity: 0,
        };

        let error = Validator::new()
            .validate_outbound(order)
            .expect_err("invalid outbound value should fail");

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

    #[cfg(feature = "jsonschema")]
    #[test]
    fn json_schema_validates_payloads() {
        use serde_json::json;

        let schema = json!({
            "type": "object",
            "required": ["order_id", "quantity"],
            "properties": {
                "order_id": { "type": "string" },
                "quantity": { "type": "integer", "minimum": 1 }
            }
        });
        let valid = json!({
            "order_id": "order-1",
            "quantity": 2
        });
        let invalid = json!({
            "order_id": "order-1",
            "quantity": 0
        });
        let validator = Validator::new();

        assert!(validator.json_schema(&schema, &valid).is_ok());

        let error = validator
            .json_schema(&schema, &invalid)
            .expect_err("payload violates schema");

        assert_eq!(error.kind(), ValidationErrorKind::Schema);
        assert!(error.message().contains("minimum"));
    }

    #[cfg(feature = "jmespath")]
    #[test]
    fn json_schema_envelope_validates_selected_payload() {
        use serde_json::json;

        let schema = json!({
            "type": "object",
            "required": ["order_id", "quantity"],
            "properties": {
                "order_id": { "type": "string" },
                "quantity": { "type": "integer", "minimum": 1 }
            }
        });
        let event = json!({
            "body": "{\"order_id\":\"order-1\",\"quantity\":2}",
            "requestContext": {"requestId": "ignored"}
        });

        Validator::new()
            .json_schema_envelope(&schema, &event, "powertools_json(body)")
            .expect("selected payload should validate");
    }

    #[cfg(feature = "jmespath")]
    #[test]
    fn json_schema_envelope_reports_envelope_errors() {
        let schema = serde_json::json!({"type": "object"});
        let error = Validator::new()
            .json_schema_envelope(&schema, &serde_json::json!({}), "body[")
            .expect_err("invalid envelope should fail");

        assert_eq!(error.kind(), ValidationErrorKind::Envelope);
        assert_eq!(error.field(), Some("envelope"));
    }
}
