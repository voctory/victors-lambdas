//! Validator facade.

use crate::ValidationError;

/// Validates decoded events and payloads.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Validator;

impl Validator {
    /// Creates a validator.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Validates that a string value is not empty.
    ///
    /// # Errors
    ///
    /// Returns an error when `value` is empty after trimming whitespace.
    pub fn required_text(&self, value: &str) -> Result<(), ValidationError> {
        if value.trim().is_empty() {
            Err(ValidationError::new("value is required"))
        } else {
            Ok(())
        }
    }
}
