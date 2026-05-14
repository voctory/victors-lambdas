//! Data masking facade.

use serde_json::Value;

use crate::{
    DataMaskingError, DataMaskingProvider, DataMaskingResult, EncryptionContext, MaskingOptions,
    mask::mask_value,
    path::matching_json_pointers,
    provider::{decode_ciphertext, encode_ciphertext},
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
    /// Field paths may be JSON Pointers such as `/customer/password`, dot paths such as
    /// `customer.password`, or JSONPath-style selectors such as `$..password`.
    ///
    /// # Errors
    ///
    /// Returns an error when a field path is invalid or when a field is missing and the
    /// missing-field policy is enabled.
    pub fn erase_fields(&self, data: Value, fields: &[&str]) -> DataMaskingResult<Value> {
        self.erase_fields_with(data, fields, &MaskingOptions::fixed())
    }

    /// Applies per-field masking strategies to selected fields.
    ///
    /// Each rule pairs a field path with the masking options used for matches at that path. Field
    /// paths may be JSON Pointers such as `/customer/password`, dot paths such as
    /// `customer.password`, or JSONPath-style selectors such as `$..password`.
    ///
    /// # Errors
    ///
    /// Returns an error when no rules are provided, when a field path is invalid, when a field is
    /// missing and the missing-field policy is enabled, or when a regex masking rule has an
    /// invalid pattern.
    pub fn erase_fields_with_rules(
        &self,
        mut data: Value,
        rules: &[(&str, MaskingOptions)],
    ) -> DataMaskingResult<Value> {
        if rules.is_empty() {
            return Err(DataMaskingError::invalid_path(""));
        }

        for (field, options) in rules {
            self.mask_field(&mut data, field, options)?;
        }

        Ok(data)
    }

    /// Applies a masking strategy to selected fields.
    ///
    /// Field paths may be JSON Pointers such as `/customer/password`, dot paths such as
    /// `customer.password`, or JSONPath-style selectors such as `$..password`.
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
            self.mask_field(&mut data, field, options)?;
        }

        Ok(data)
    }

    fn mask_field(
        self,
        data: &mut Value,
        field: &str,
        options: &MaskingOptions,
    ) -> DataMaskingResult<()> {
        let pointers = matching_json_pointers(data, field)?;
        if pointers.is_empty() {
            if self.config.raise_on_missing_field() {
                return Err(DataMaskingError::missing_field(field));
            }
            return Ok(());
        }

        for pointer in pointers {
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

        Ok(())
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

    /// Encrypts a JSON value with a provider and returns base64-encoded ciphertext.
    ///
    /// # Errors
    ///
    /// Returns an error when JSON serialization fails or the provider cannot encrypt the payload.
    pub fn encrypt<P>(&self, data: &Value, provider: &mut P) -> DataMaskingResult<String>
    where
        P: DataMaskingProvider,
    {
        self.encrypt_with_context(data, provider, &EncryptionContext::new())
    }

    /// Encrypts a JSON value with an authenticated encryption context.
    ///
    /// # Errors
    ///
    /// Returns an error when JSON serialization fails or the provider cannot encrypt the payload.
    pub fn encrypt_with_context<P>(
        &self,
        data: &Value,
        provider: &mut P,
        encryption_context: &EncryptionContext,
    ) -> DataMaskingResult<String>
    where
        P: DataMaskingProvider,
    {
        let plaintext = serde_json::to_vec(data).map_err(DataMaskingError::json)?;
        let ciphertext = provider.encrypt(&plaintext, encryption_context)?;
        Ok(encode_ciphertext(&ciphertext))
    }

    /// Decrypts base64-encoded ciphertext into a JSON value.
    ///
    /// # Errors
    ///
    /// Returns an error when ciphertext decoding fails, the provider cannot decrypt the payload,
    /// or decrypted plaintext is not valid JSON.
    pub fn decrypt<P>(&self, ciphertext: &str, provider: &mut P) -> DataMaskingResult<Value>
    where
        P: DataMaskingProvider,
    {
        self.decrypt_with_context(ciphertext, provider, &EncryptionContext::new())
    }

    /// Decrypts base64-encoded ciphertext with an authenticated encryption context.
    ///
    /// # Errors
    ///
    /// Returns an error when ciphertext decoding fails, the provider cannot decrypt the payload,
    /// or decrypted plaintext is not valid JSON.
    pub fn decrypt_with_context<P>(
        &self,
        ciphertext: &str,
        provider: &mut P,
        encryption_context: &EncryptionContext,
    ) -> DataMaskingResult<Value>
    where
        P: DataMaskingProvider,
    {
        let ciphertext = decode_ciphertext(ciphertext)?;
        let plaintext = provider.decrypt(&ciphertext, encryption_context)?;
        serde_json::from_slice(&plaintext).map_err(DataMaskingError::json)
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

/// Applies per-field masking strategies to selected fields.
///
/// # Errors
///
/// Returns an error when no rules are provided, a field path is invalid, a field is missing, or a
/// regex masking rule has an invalid pattern.
pub fn erase_fields_with_rules(
    data: Value,
    rules: &[(&str, MaskingOptions)],
) -> DataMaskingResult<Value> {
    DataMasking::new().erase_fields_with_rules(data, rules)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[derive(Debug, Default)]
    struct ReversingProvider {
        encrypt_context: Option<EncryptionContext>,
        decrypt_context: Option<EncryptionContext>,
    }

    impl DataMaskingProvider for ReversingProvider {
        fn encrypt(
            &mut self,
            plaintext: &[u8],
            encryption_context: &EncryptionContext,
        ) -> DataMaskingResult<Vec<u8>> {
            self.encrypt_context = Some(encryption_context.clone());
            let mut ciphertext = plaintext.to_vec();
            ciphertext.reverse();
            Ok(ciphertext)
        }

        fn decrypt(
            &mut self,
            ciphertext: &[u8],
            encryption_context: &EncryptionContext,
        ) -> DataMaskingResult<Vec<u8>> {
            self.decrypt_context = Some(encryption_context.clone());
            let mut plaintext = ciphertext.to_vec();
            plaintext.reverse();
            Ok(plaintext)
        }
    }

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
    fn applies_per_field_masking_rules() {
        let data = json!({
            "customer": {
                "name": "Ada",
                "password": "secret",
                "phone": "555-0100",
                "cards": [
                    {"number": "4111111111111111"},
                    {"number": "5555555555554444"}
                ]
            }
        });
        let rules = [
            ("customer.password", MaskingOptions::fixed()),
            ("customer.phone", MaskingOptions::dynamic()),
            ("customer.name", MaskingOptions::custom("REDACTED")),
            (
                "$.customer.cards[*].number",
                MaskingOptions::regex(r"\d{12}(\d{4})", "************$1"),
            ),
        ];

        let masked = DataMasking::new()
            .erase_fields_with_rules(data, &rules)
            .expect("rules should be applied");

        assert_eq!(masked["customer"]["password"], json!("*****"));
        assert_eq!(masked["customer"]["phone"], json!("***-****"));
        assert_eq!(masked["customer"]["name"], json!("RED"));
        assert_eq!(
            masked["customer"]["cards"][0]["number"],
            json!("************1111")
        );
        assert_eq!(
            masked["customer"]["cards"][1]["number"],
            json!("************4444")
        );
    }

    #[test]
    fn empty_per_field_masking_rules_error() {
        let error = DataMasking::new()
            .erase_fields_with_rules(json!({}), &[])
            .expect_err("empty rules should fail");

        assert_eq!(error.kind(), crate::DataMaskingErrorKind::InvalidPath);
    }

    #[test]
    fn erases_fields_by_jsonpath_wildcard() {
        let data = json!({
            "customer": {
                "cards": [
                    {"number": "4111111111111111"},
                    {"number": "5555555555554444"}
                ]
            }
        });

        let masked = DataMasking::new()
            .erase_fields(data, &["$.customer.cards[*].number"])
            .expect("matching fields should be masked");

        assert_eq!(masked["customer"]["cards"][0]["number"], json!("*****"));
        assert_eq!(masked["customer"]["cards"][1]["number"], json!("*****"));
    }

    #[test]
    fn erases_fields_by_jsonpath_recursive_descent() {
        let data = json!({
            "headers": {"authorization": "top-secret"},
            "records": [
                {"payload": {"authorization": "nested-secret"}}
            ]
        });

        let masked = DataMasking::new()
            .erase_fields(data, &["$..authorization"])
            .expect("matching fields should be masked");

        assert_eq!(masked["headers"]["authorization"], json!("*****"));
        assert_eq!(
            masked["records"][0]["payload"]["authorization"],
            json!("*****")
        );
    }

    #[test]
    fn erases_fields_by_jsonpath_filter() {
        let data = json!({
            "addresses": [
                {"postcode": 90210, "line": "private"},
                {"postcode": 1000, "line": "public"}
            ]
        });

        let masked = DataMasking::new()
            .erase_fields(data, &["$.addresses[?(@.postcode > 12000)].line"])
            .expect("matching fields should be masked");

        assert_eq!(masked["addresses"][0]["line"], json!("*****"));
        assert_eq!(masked["addresses"][1]["line"], json!("public"));
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

    #[test]
    fn encrypts_and_decrypts_json_with_provider() {
        let data = json!({"customer": {"password": "secret"}});
        let mut provider = ReversingProvider::default();
        let data_masking = DataMasking::new();

        let ciphertext = data_masking
            .encrypt(&data, &mut provider)
            .expect("encrypt should succeed");
        let plaintext = data_masking
            .decrypt(&ciphertext, &mut provider)
            .expect("decrypt should succeed");

        assert_ne!(ciphertext, data.to_string());
        assert_eq!(plaintext, data);
    }

    #[test]
    fn passes_encryption_context_to_provider() {
        let data = json!({"tenant": "one"});
        let mut provider = ReversingProvider::default();
        let data_masking = DataMasking::new();
        let mut context = EncryptionContext::new();
        context.insert("tenant".to_string(), "one".to_string());

        let ciphertext = data_masking
            .encrypt_with_context(&data, &mut provider, &context)
            .expect("encrypt should succeed");
        let _plaintext = data_masking
            .decrypt_with_context(&ciphertext, &mut provider, &context)
            .expect("decrypt should succeed");

        assert_eq!(provider.encrypt_context.as_ref(), Some(&context));
        assert_eq!(provider.decrypt_context.as_ref(), Some(&context));
    }

    #[test]
    fn decrypt_rejects_invalid_ciphertext_encoding() {
        let mut provider = ReversingProvider::default();
        let error = DataMasking::new()
            .decrypt("not base64", &mut provider)
            .expect_err("invalid ciphertext should fail");

        assert_eq!(error.kind(), crate::DataMaskingErrorKind::Decrypt);
    }
}
