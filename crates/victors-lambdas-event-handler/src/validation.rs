//! Event handler validation integration.

use victors_lambdas_validation::{ValidationError, ValidationResult};

use crate::{Request, Response};

/// Function signature used by request validators.
pub type RequestValidator = dyn Fn(&Request) -> ValidationResult + Send + Sync + 'static;

/// Function signature used by response validators.
pub type ResponseValidator =
    dyn Fn(&Request, &Response) -> ValidationResult + Send + Sync + 'static;

/// Validation hooks for event-handler routers.
///
/// Request validators run after route matching and path parameter capture, but
/// before the matched handler runs. Response validators run after response
/// middleware and before CORS headers are applied.
#[derive(Default)]
pub struct ValidationConfig {
    request_validators: Vec<Box<RequestValidator>>,
    response_validators: Vec<Box<ResponseValidator>>,
}

impl ValidationConfig {
    /// Creates an empty validation configuration.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            request_validators: Vec::new(),
            response_validators: Vec::new(),
        }
    }

    /// Returns a copy with a request validator appended.
    #[must_use]
    pub fn with_request_validator(
        mut self,
        validator: impl Fn(&Request) -> ValidationResult + Send + Sync + 'static,
    ) -> Self {
        self.add_request_validator(validator);
        self
    }

    /// Returns a copy with a response validator appended.
    #[must_use]
    pub fn with_response_validator(
        mut self,
        validator: impl Fn(&Request, &Response) -> ValidationResult + Send + Sync + 'static,
    ) -> Self {
        self.add_response_validator(validator);
        self
    }

    /// Adds a request validator.
    pub fn add_request_validator(
        &mut self,
        validator: impl Fn(&Request) -> ValidationResult + Send + Sync + 'static,
    ) -> &mut Self {
        self.request_validators.push(Box::new(validator));
        self
    }

    /// Adds a response validator.
    pub fn add_response_validator(
        &mut self,
        validator: impl Fn(&Request, &Response) -> ValidationResult + Send + Sync + 'static,
    ) -> &mut Self {
        self.response_validators.push(Box::new(validator));
        self
    }

    /// Returns the number of registered request validators.
    #[must_use]
    pub fn request_validators_len(&self) -> usize {
        self.request_validators.len()
    }

    /// Returns the number of registered response validators.
    #[must_use]
    pub fn response_validators_len(&self) -> usize {
        self.response_validators.len()
    }

    pub(crate) fn validate_request(&self, request: &Request) -> ValidationResult {
        for validator in &self.request_validators {
            validator(request)?;
        }

        Ok(())
    }

    pub(crate) fn validate_response(
        &self,
        request: &Request,
        response: &Response,
    ) -> ValidationResult {
        for validator in &self.response_validators {
            validator(request, response)?;
        }

        Ok(())
    }

    pub(crate) fn append(&mut self, other: Self) {
        self.request_validators.extend(other.request_validators);
        self.response_validators.extend(other.response_validators);
    }
}

pub(crate) fn request_validation_response(error: &ValidationError) -> Response {
    validation_error_response(422, "Request validation failed", error)
}

pub(crate) fn response_validation_response(error: &ValidationError) -> Response {
    validation_error_response(500, "Response validation failed", error)
}

fn validation_error_response(status_code: u16, summary: &str, error: &ValidationError) -> Response {
    Response::new(status_code)
        .with_header("content-type", "text/plain")
        .with_body(format!("{summary}: {}", error.message()))
}
