//! Idempotency keys.

/// Stable key used to deduplicate handler work.
#[derive(Clone, Debug, Eq, PartialEq)]
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
}
