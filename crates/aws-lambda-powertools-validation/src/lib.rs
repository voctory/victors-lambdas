//! Validation utility.

mod error;
#[cfg(feature = "jsonschema")]
mod schema;
mod validator;

pub use error::{ValidationError, ValidationErrorKind};
#[cfg(feature = "jsonschema")]
pub use schema::JsonSchemaCache;
pub use validator::{Validate, ValidationResult, Validator};
