//! Masking strategies and value transformations.

use regex::Regex;
use serde_json::{Number, Value};

use crate::{DataMaskingError, DataMaskingResult};

/// Default replacement string for erased values.
pub const DATA_MASKING_STRING: &str = "*****";

/// Strategy used to mask selected values.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MaskingStrategy {
    /// Replace the whole value with `*****`.
    Fixed,
    /// Replace non-separator characters with `*`, preserving `-`, `_`, `.`, and spaces.
    Dynamic,
    /// Replace primitive values with a caller-provided mask.
    Custom(String),
    /// Replace regex matches in primitive values with a caller-provided replacement.
    Regex {
        /// Regular expression used to find sensitive fragments.
        pattern: String,
        /// Replacement applied to each match.
        replacement: String,
    },
}

impl Default for MaskingStrategy {
    fn default() -> Self {
        Self::Fixed
    }
}

/// Options used when masking data.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MaskingOptions {
    strategy: MaskingStrategy,
}

impl MaskingOptions {
    /// Creates options that replace values with `*****`.
    #[must_use]
    pub const fn fixed() -> Self {
        Self {
            strategy: MaskingStrategy::Fixed,
        }
    }

    /// Creates options that mask non-separator characters with `*`.
    #[must_use]
    pub const fn dynamic() -> Self {
        Self {
            strategy: MaskingStrategy::Dynamic,
        }
    }

    /// Creates options that replace primitive values with a caller-provided mask.
    #[must_use]
    pub fn custom(mask: impl Into<String>) -> Self {
        Self {
            strategy: MaskingStrategy::Custom(mask.into()),
        }
    }

    /// Creates options that replace regex matches in primitive values.
    #[must_use]
    pub fn regex(pattern: impl Into<String>, replacement: impl Into<String>) -> Self {
        Self {
            strategy: MaskingStrategy::Regex {
                pattern: pattern.into(),
                replacement: replacement.into(),
            },
        }
    }

    /// Returns the configured masking strategy.
    #[must_use]
    pub const fn strategy(&self) -> &MaskingStrategy {
        &self.strategy
    }
}

pub(crate) fn mask_value(value: Value, options: &MaskingOptions) -> DataMaskingResult<Value> {
    match options.strategy() {
        MaskingStrategy::Fixed => Ok(Value::String(DATA_MASKING_STRING.to_string())),
        MaskingStrategy::Dynamic => Ok(mask_dynamic_value(value)),
        MaskingStrategy::Custom(mask) => Ok(mask_custom_value(value, mask)),
        MaskingStrategy::Regex {
            pattern,
            replacement,
        } => mask_regex_value(value, pattern, replacement),
    }
}

fn mask_dynamic_value(value: Value) -> Value {
    match value {
        Value::Array(values) => Value::Array(values.into_iter().map(mask_dynamic_value).collect()),
        Value::Object(values) => Value::Object(
            values
                .into_iter()
                .map(|(key, value)| (key, mask_dynamic_value(value)))
                .collect(),
        ),
        Value::String(value) => Value::String(mask_dynamic_text(&value)),
        Value::Number(value) => Value::String(mask_dynamic_text(&value.to_string())),
        Value::Bool(value) => Value::String(mask_dynamic_text(bool_text(value))),
        Value::Null => Value::String(DATA_MASKING_STRING.to_string()),
    }
}

fn mask_custom_value(value: Value, mask: &str) -> Value {
    match value {
        Value::Array(values) => Value::Array(
            values
                .into_iter()
                .map(|value| mask_custom_value(value, mask))
                .collect(),
        ),
        Value::Object(values) => Value::Object(
            values
                .into_iter()
                .map(|(key, value)| (key, mask_custom_value(value, mask)))
                .collect(),
        ),
        Value::String(value) => Value::String(mask_custom_text(&value, mask)),
        Value::Number(value) => Value::String(mask_custom_number(&value, mask)),
        Value::Bool(value) => Value::String(mask_custom_text(bool_text(value), mask)),
        Value::Null => Value::String(mask.to_string()),
    }
}

fn mask_regex_value(value: Value, pattern: &str, replacement: &str) -> DataMaskingResult<Value> {
    let regex = Regex::new(pattern).map_err(|error| DataMaskingError::regex(pattern, error))?;
    Ok(apply_regex_value(value, &regex, replacement))
}

fn apply_regex_value(value: Value, regex: &Regex, replacement: &str) -> Value {
    match value {
        Value::Array(values) => Value::Array(
            values
                .into_iter()
                .map(|value| apply_regex_value(value, regex, replacement))
                .collect(),
        ),
        Value::Object(values) => Value::Object(
            values
                .into_iter()
                .map(|(key, value)| (key, apply_regex_value(value, regex, replacement)))
                .collect(),
        ),
        Value::String(value) => Value::String(regex.replace_all(&value, replacement).into_owned()),
        Value::Number(value) => Value::String(
            regex
                .replace_all(&value.to_string(), replacement)
                .into_owned(),
        ),
        Value::Bool(value) => Value::String(
            regex
                .replace_all(bool_text(value), replacement)
                .into_owned(),
        ),
        Value::Null => Value::String(DATA_MASKING_STRING.to_string()),
    }
}

fn mask_dynamic_text(value: &str) -> String {
    if value.is_empty() {
        return String::new();
    }

    value
        .chars()
        .map(|character| {
            if matches!(character, '-' | '_' | '.' | ' ') {
                character
            } else {
                '*'
            }
        })
        .collect()
}

fn mask_custom_number(value: &Number, mask: &str) -> String {
    mask_custom_text(&value.to_string(), mask)
}

fn mask_custom_text(value: &str, mask: &str) -> String {
    if mask.chars().count() >= value.chars().count() {
        mask.chars().take(value.chars().count()).collect()
    } else {
        mask.to_string()
    }
}

fn bool_text(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn fixed_strategy_replaces_any_value() {
        let masked = mask_value(json!({"secret": "value"}), &MaskingOptions::fixed())
            .expect("masking should succeed");

        assert_eq!(masked, json!("*****"));
    }

    #[test]
    fn dynamic_strategy_preserves_separators() {
        let masked = mask_value(json!("abc-12_3. z"), &MaskingOptions::dynamic())
            .expect("masking should succeed");

        assert_eq!(masked, json!("***-**_*. *"));
    }

    #[test]
    fn custom_strategy_truncates_long_masks() {
        let masked = mask_value(json!("abc"), &MaskingOptions::custom("XXXXXXXX"))
            .expect("masking should succeed");

        assert_eq!(masked, json!("XXX"));
    }

    #[test]
    fn custom_strategy_uses_short_masks_as_is() {
        let masked = mask_value(json!("abcdef"), &MaskingOptions::custom("MASK"))
            .expect("masking should succeed");

        assert_eq!(masked, json!("MASK"));
    }

    #[test]
    fn regex_strategy_replaces_matches() {
        let masked = mask_value(
            json!("card 4111111111111111"),
            &MaskingOptions::regex(r"\d{12}(\d{4})", "************$1"),
        )
        .expect("masking should succeed");

        assert_eq!(masked, json!("card ************1111"));
    }
}
