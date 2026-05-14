//! Parser-integrated batch records.

/// A parsed batch record with the original source record.
#[derive(Debug, Eq, PartialEq)]
pub struct ParsedBatchRecord<'a, T, R> {
    item_identifier: String,
    payload: T,
    raw_record: &'a R,
}

impl<'a, T, R> ParsedBatchRecord<'a, T, R> {
    /// Creates a parsed batch record.
    #[must_use]
    pub fn new(item_identifier: impl Into<String>, payload: T, raw_record: &'a R) -> Self {
        Self {
            item_identifier: item_identifier.into(),
            payload,
            raw_record,
        }
    }

    /// Returns the item identifier used for partial batch responses.
    #[must_use]
    pub fn item_identifier(&self) -> &str {
        &self.item_identifier
    }

    /// Returns the parsed payload.
    #[must_use]
    pub const fn payload(&self) -> &T {
        &self.payload
    }

    /// Returns the original source record.
    #[must_use]
    pub const fn raw_record(&self) -> &'a R {
        self.raw_record
    }

    /// Consumes the parsed record and returns the parsed payload.
    #[must_use]
    pub fn into_payload(self) -> T {
        self.payload
    }
}
