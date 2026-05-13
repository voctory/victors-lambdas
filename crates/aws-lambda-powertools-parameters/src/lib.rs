//! Parameter retrieval utility.

mod async_parameter;
mod cache;
mod parameter;
mod provider;
#[cfg(feature = "secrets")]
mod secrets;
#[cfg(feature = "ssm")]
mod ssm;
mod transform;

pub use async_parameter::{
    AsyncParameterError, AsyncParameterProvider, AsyncParameterResult, AsyncParameters,
    ParameterFuture, ParameterProviderError, ParameterProviderResult,
};
pub use cache::CachePolicy;
pub use parameter::{Parameter, Parameters};
pub use provider::{InMemoryParameterProvider, ParameterProvider};
#[cfg(feature = "secrets")]
pub use secrets::SecretsManagerProvider;
#[cfg(feature = "ssm")]
pub use ssm::{SsmParameterProvider, SsmParametersByName};
pub use transform::{ParameterTransformError, ParameterTransformErrorKind};
