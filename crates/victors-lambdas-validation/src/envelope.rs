//! Envelope extraction for validation.

use serde_json::Value;

use crate::ValidationError;

/// Extracts a JSON value from an event with a `JMESPath` envelope expression.
///
/// Powertools `JMESPath` helper functions such as `powertools_json`,
/// `powertools_base64`, and `powertools_base64_gzip` are available in the
/// expression.
///
/// # Errors
///
/// Returns a validation error when the envelope expression is blank or when it
/// cannot be compiled or evaluated.
#[cfg(feature = "jmespath")]
pub fn extract_envelope(event: &Value, envelope: &str) -> Result<Value, ValidationError> {
    if envelope.trim().is_empty() {
        return Err(ValidationError::envelope("envelope expression is empty"));
    }

    victors_lambdas_jmespath::search(envelope, event)
        .map_err(|error| ValidationError::envelope(error.to_string()))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::extract_envelope;
    use crate::ValidationErrorKind;

    #[test]
    fn extracts_json_payload_from_envelope() {
        let event = json!({
            "body": "{\"order_id\":\"order-1\",\"quantity\":2}",
        });

        let payload =
            extract_envelope(&event, "powertools_json(body)").expect("envelope should extract");

        assert_eq!(payload, json!({"order_id": "order-1", "quantity": 2}));
    }

    #[test]
    fn rejects_blank_envelope() {
        let error =
            extract_envelope(&json!({}), " ").expect_err("blank envelope expressions should fail");

        assert_eq!(error.kind(), ValidationErrorKind::Envelope);
        assert_eq!(error.message(), "envelope expression is empty");
    }
}
