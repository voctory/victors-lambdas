//! Payload hashing and key extraction helpers.

use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::{IdempotencyError, IdempotencyKey, IdempotencyResult};

/// Hashes a JSON-serializable payload with SHA-256.
///
/// The payload is first converted to `serde_json::Value` and then serialized to
/// bytes so object keys use `serde_json`'s deterministic ordering.
///
/// # Errors
///
/// Returns an error when the payload cannot be represented as JSON.
pub fn hash_payload<T>(payload: &T) -> IdempotencyResult<String>
where
    T: Serialize + ?Sized,
{
    let value = serde_json::to_value(payload)
        .map_err(|error| IdempotencyError::serialization(error.to_string()))?;
    hash_json_value(&value)
}

/// Builds an idempotency key from the SHA-256 hash of a JSON-serializable payload.
///
/// # Errors
///
/// Returns an error when the payload cannot be represented as JSON.
pub fn key_from_payload<T>(payload: &T) -> IdempotencyResult<IdempotencyKey>
where
    T: Serialize + ?Sized,
{
    Ok(IdempotencyKey::new(hash_payload(payload)?))
}

/// Builds an idempotency key from a value selected with a JSON Pointer.
///
/// The selected value is hashed rather than used directly, matching Powertools
/// idempotency behavior where keys are hash representations of payloads or
/// payload subsets.
///
/// # Errors
///
/// Returns an error when the pointer does not select a non-empty value or when
/// the selected value cannot be represented as JSON.
pub fn key_from_json_pointer(payload: &Value, pointer: &str) -> IdempotencyResult<IdempotencyKey> {
    let selected = if pointer.is_empty() {
        Some(payload)
    } else {
        payload.pointer(pointer)
    }
    .filter(|value| !is_empty_value(value))
    .ok_or(IdempotencyError::MissingKey)?;

    Ok(IdempotencyKey::new(hash_json_value(selected)?))
}

fn hash_json_value(value: &Value) -> IdempotencyResult<String> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| IdempotencyError::serialization(error.to_string()))?;
    let digest = Sha256::digest(bytes);
    Ok(hex_lower(&digest))
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn is_empty_value(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::String(value) => value.is_empty(),
        Value::Array(value) => value.is_empty(),
        Value::Object(value) => value.is_empty(),
        Value::Bool(_) | Value::Number(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{hash_payload, key_from_json_pointer, key_from_payload};
    use crate::{IdempotencyError, IdempotencyKey};

    #[test]
    fn hash_payload_uses_stable_sha256_hex() {
        let hash = hash_payload(&json!({"order_id": "abc"})).expect("payload hashes");

        assert_eq!(
            hash,
            "bb6f3f83563393f26ac52e0cce152cb209644dc4588593328409d719082b8669"
        );
    }

    #[test]
    fn key_from_payload_wraps_hash() {
        let key = key_from_payload(&json!({"order_id": "abc"})).expect("payload hashes");

        assert_eq!(
            key,
            IdempotencyKey::new("bb6f3f83563393f26ac52e0cce152cb209644dc4588593328409d719082b8669")
        );
    }

    #[test]
    fn key_from_json_pointer_hashes_selected_payload_subset() {
        let payload = json!({
            "body": {
                "order_id": "abc",
                "timestamp": "ignored",
            },
        });

        let key = key_from_json_pointer(&payload, "/body/order_id").expect("key exists");
        let expected = key_from_payload(&"abc").expect("payload hashes");

        assert_eq!(key, expected);
    }

    #[test]
    fn key_from_json_pointer_rejects_empty_selection() {
        let payload = json!({"body": {}});

        assert_eq!(
            key_from_json_pointer(&payload, "/body"),
            Err(IdempotencyError::MissingKey)
        );
    }
}
