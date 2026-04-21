//! Testing helpers for Powertools Lambda Rust.

mod context;

pub use aws_lambda_powertools_parameters::InMemoryParameterProvider as ParameterProviderStub;
pub use context::LambdaContextStub;
