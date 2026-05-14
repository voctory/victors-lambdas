//! Data masking facade.

use serde_json::Value;

use crate::{
    DataMaskingError, DataMaskingResult, MaskingOptions, mask::mask_value, path::to_json_pointer,
};

/// Configuration for the data masking facade.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DataMaskingConfig {
    raise_on_missing_field: bool,
}

impl Default for DataMaskingConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl DataMaskingConfig {
    /// Creates data masking configuration with missing fields treated as errors.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            raise_on_missing_field: true,
        }
    }

    /// Sets whether missing field paths should return an error.
    #[must_use]
    pub const fn with_raise_on_missing_field(mut self, raise: bool) -> Self {
        self.raise_on_missing_field = raise;
        self
    }

    /// Returns whether missing field paths should return an error.
    #[must_use]
    pub const fn raise_on_missing_field(&self) -> bool {
        self.raise_on_missing_field
    }
}

/// Data masking utility for `serde_json::Value` payloads.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DataMasking {
    config: DataMaskingConfig,
}

impl Default for DataMasking {
    fn default() -> Self {
        Self::new()
    }
}

impl DataMasking {
    /// Creates a data masking utility with default configuration.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            config: DataMaskingConfig::new(),
        }
    }

    /// Creates a data masking utility with explicit configuration.
    #[must_use]
    pub const fn with_config(config: DataMaskingConfig) -> Self {
        Self { config }
    }

    /// Returns the configured missing-field policy.
    #[must_use]
    pub const fn config(&self) -> DataMaskingConfig {
        self.config
    }

    /// Replaces an entire JSON value with the default mask.
    ///
    /// This mirrors the default Powertools data masking behavior for whole-value erasure.
    #[must_use]
    pub fn erase(&self, _data: Value) -> Value {
        Value::String(crate::DATA_MASKING_STRING.to_string())
    }

    /// Applies a masking strategy to an entire JSON value.
    ///
    /// # Errors
    ///
    /// Returns an error when regex masking is requested with an invalid regex pattern.
    pub fn erase_with(&self, data: Value, options: &MaskingOptions) -> DataMaskingResult<Value> {
        mask_value(data, options)
    }

    /// Replaces selected fields with the default mask.
    ///
    /// Field paths may be JSON Pointers such as `/customer/password` or dot paths such as
    /// `customer.password`.
    ///
    /// # Errors
    ///
    /// Returns an error when a field path is invalid or when a field is missing and the
    /// missing-field policy is enabled.
    pub fn erase_fields(&self, data: Value, fields: &[&str]) -> DataMaskingResult<Value> {
        self.erase_fields_with(data, fields, &MaskingOptions::fixed())
    }

    /// Applies a masking strategy to selected fields.
    ///
    /// Field paths may be JSON Pointers such as `/customer/password` or dot paths such as
    /// `customer.password`.
    ///
    /// # Errors
    ///
    /// Returns an error when a field path is invalid, when a field is missing and the
    /// missing-field policy is enabled, or when regex masking is requested with an invalid regex
    /// pattern.
    pub fn erase_fields_with(
        &self,
        mut data: Value,
        fields: &[&str],
        options: &MaskingOptions,
    ) -> DataMaskingResult<Value> {
        if fields.is_empty() {
            return Err(DataMaskingError::invalid_path(""));
        }

        for field in fields {
            let pointer = to_json_pointer(field)?;
            match data.pointer_mut(&pointer) {
                Some(value) => {
                    let masked = mask_value(value.take(), options)?;
                    *value = masked;
                }
                None if self.config.raise_on_missing_field() => {
                    return Err(DataMaskingError::missing_field(field));
                }
                None => {}
            }
        }

        Ok(data)
    }

    /// Parses a JSON string and replaces selected fields with the default mask.
    ///
    /// # Errors
    ///
    /// Returns an error when the input is not valid JSON, a field path is invalid, or a field is
    /// missing and the missing-field policy is enabled.
    pub fn erase_json_str(&self, data: &str, fields: &[&str]) -> DataMaskingResult<Value> {
        let value = serde_json::from_str(data).map_err(DataMaskingError::json)?;
        self.erase_fields(value, fields)
    }
}

/// Replaces an entire JSON value with the default mask.
#[must_use]
pub fn erase(data: Value) -> Value {
    DataMasking::new().erase(data)
}

/// Replaces selected fields with the default mask.
///
/// # Errors
///
/// Returns an error when a field path is invalid or when a field is missing.
pub fn erase_fields(data: Value, fields: &[&str]) -> DataMaskingResult<Value> {
    DataMasking::new().erase_fields(data, fields)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn erases_entire_payload() {
        let masked = DataMasking::new().erase(json!({"password": "secret"}));

        assert_eq!(masked, json!("*****"));
    }

    #[test]
    fn erases_fields_by_dot_path_and_json_pointer() {
        let data = json!({
            "customer": {
                "name": "Ada",
                "password": "secret",
                "cards": [{"number": "4111111111111111"}]
            }
        });

        let masked = DataMasking::new()
            .erase_fields(data, &["customer.password", "/customer/cards/0/number"])
            .expect("fields should be masked");

        assert_eq!(masked["customer"]["name"], json!("Ada"));
        assert_eq!(masked["customer"]["password"], json!("*****"));
        assert_eq!(masked["customer"]["cards"][0]["number"], json!("*****"));
    }

    #[test]
    fn applies_dynamic_masking_to_fields() {
        let data = json!({
            "customer": {
                "phone": "555-0100"
            }
        });

        let masked = DataMasking::new()
            .erase_fields_with(data, &["customer.phone"], &MaskingOptions::dynamic())
            .expect("field should be masked");

        assert_eq!(masked["customer"]["phone"], json!("***-****"));
    }

    #[test]
    fn applies_regex_masking_to_fields() {
        let data = json!({
            "customer": {
                "card": "4111111111111111"
            }
        });

        let masked = DataMasking::new()
            .erase_fields_with(
                data,
                &["customer.card"],
                &MaskingOptions::regex(r"\d{12}(\d{4})", "************$1"),
            )
            .expect("field should be masked");

        assert_eq!(masked["customer"]["card"], json!("************1111"));
    }

    #[test]
    fn missing_fields_error_by_default() {
        let error = DataMasking::new()
            .erase_fields(json!({}), &["customer.password"])
            .expect_err("missing field should fail");

        assert_eq!(error.kind(), crate::DataMaskingErrorKind::MissingField);
    }

    #[test]
    fn missing_fields_can_be_ignored() {
        let data = json!({"customer": {"name": "Ada"}});
        let data_masking =
            DataMasking::with_config(DataMaskingConfig::new().with_raise_on_missing_field(false));

        let masked = data_masking
            .erase_fields(data.clone(), &["customer.password"])
            .expect("missing field should be ignored");

        assert_eq!(masked, data);
    }

    #[test]
    fn parses_json_strings() {
        let masked = DataMasking::new()
            .erase_json_str(
                r#"{"customer":{"password":"secret"}}"#,
                &["customer.password"],
            )
            .expect("JSON string should be masked");

        assert_eq!(masked["customer"]["password"], json!("*****"));
    }
}
