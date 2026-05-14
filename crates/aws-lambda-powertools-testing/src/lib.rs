//! Testing helpers for Powertools Lambda Rust.

mod context;
mod fixture;
mod handler;
#[cfg(feature = "streaming")]
mod streaming;

pub use aws_lambda_powertools_parameters::InMemoryParameterProvider as ParameterProviderStub;
pub use context::LambdaContextStub;
pub use fixture::{FixtureError, load_json_fixture, read_fixture, read_fixture_bytes};
pub use handler::HandlerHarness;
#[cfg(feature = "streaming")]
pub use streaming::S3ObjectClientStub;
