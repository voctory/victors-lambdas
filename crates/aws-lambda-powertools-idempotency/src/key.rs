//! Idempotency keys.

use std::fmt;

use serde::Serialize;
use serde_json::Value;

use crate::{IdempotencyResult, key_from_json_pointer, key_from_payload};

/// Stable key used to deduplicate handler work.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct IdempotencyKey {
    value: String,
}

impl IdempotencyKey {
    /// Creates an idempotency key.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
        }
    }

    /// Returns the key value.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Consumes the key and returns the underlying string.
    #[must_use]
    pub fn into_string(self) -> String {
        self.value
    }

    /// Returns whether the key value is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.value.is_empty()
    }

    /// Creates an idempotency key from a hashed JSON-serializable payload.
    ///
    /// # Errors
    ///
    /// Returns an error when the payload cannot be represented as JSON.
    pub fn from_payload<T>(payload: &T) -> IdempotencyResult<Self>
    where
        T: Serialize + ?Sized,
    {
        key_from_payload(payload)
    }

    /// Creates an idempotency key from a hashed JSON Pointer selection.
    ///
    /// # Errors
    ///
    /// Returns an error when the pointer does not select a non-empty value or
    /// when the selected value cannot be represented as JSON.
    pub fn from_json_pointer(payload: &Value, pointer: &str) -> IdempotencyResult<Self> {
        key_from_json_pointer(payload, pointer)
    }
}

impl fmt::Display for IdempotencyKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.value)
    }
}

impl From<String> for IdempotencyKey {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for IdempotencyKey {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::IdempotencyKey;

    #[test]
    fn key_can_be_used_as_ordered_map_key() {
        let mut records = BTreeMap::new();

        records.insert(IdempotencyKey::new("b"), 2);
        records.insert(IdempotencyKey::new("a"), 1);

        let keys = records
            .keys()
            .map(IdempotencyKey::value)
            .collect::<Vec<_>>();
        assert_eq!(keys, vec!["a", "b"]);
    }

    #[test]
    fn key_exposes_display_and_owned_value() {
        let key = IdempotencyKey::from("request-1");

        assert_eq!(key.to_string(), "request-1");
        assert_eq!(key.into_string(), "request-1");
    }

    #[test]
    fn key_can_be_derived_from_payload_hash() {
        let key = IdempotencyKey::from_payload(&serde_json::json!({"request": 1}))
            .expect("payload hashes");

        assert!(!key.is_empty());
    }
}
