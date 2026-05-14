//! Validation utility.

#[cfg(feature = "jmespath")]
mod envelope;
mod error;
#[cfg(feature = "jsonschema")]
mod schema;
mod validator;

#[cfg(feature = "jmespath")]
pub use envelope::extract_envelope;
pub use error::{ValidationError, ValidationErrorKind};
#[cfg(feature = "jsonschema")]
pub use schema::JsonSchemaCache;
pub use validator::{Validate, ValidationResult, Validator};
