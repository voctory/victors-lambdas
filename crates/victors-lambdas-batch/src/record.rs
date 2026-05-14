//! Batch records.

/// A batch record with an identifier and payload.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatchRecord<T> {
    item_identifier: String,
    payload: T,
}

impl<T> BatchRecord<T> {
    /// Creates a batch record.
    #[must_use]
    pub fn new(item_identifier: impl Into<String>, payload: T) -> Self {
        Self {
            item_identifier: item_identifier.into(),
            payload,
        }
    }

    /// Returns the item identifier.
    #[must_use]
    pub fn item_identifier(&self) -> &str {
        &self.item_identifier
    }

    /// Returns the payload.
    #[must_use]
    pub fn payload(&self) -> &T {
        &self.payload
    }

    /// Consumes the record and returns the payload.
    #[must_use]
    pub fn into_payload(self) -> T {
        self.payload
    }
}
