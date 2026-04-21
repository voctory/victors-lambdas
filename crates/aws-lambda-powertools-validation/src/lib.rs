//! Validation utility.

mod error;
mod validator;

pub use error::{ValidationError, ValidationErrorKind};
pub use validator::{Validate, ValidationResult, Validator};
