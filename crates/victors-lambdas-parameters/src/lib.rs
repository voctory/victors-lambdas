//! Parameter retrieval utility.

#[cfg(feature = "appconfig")]
mod appconfig;
mod async_parameter;
mod cache;
#[cfg(feature = "dynamodb")]
mod dynamodb;
mod parameter;
mod provider;
#[cfg(feature = "secrets")]
mod secrets;
#[cfg(feature = "ssm")]
mod ssm;
mod transform;

#[cfg(feature = "appconfig")]
pub use appconfig::AppConfigProvider;
pub use async_parameter::{
    AsyncParameterError, AsyncParameterProvider, AsyncParameterResult, AsyncParameters,
    ParameterFuture, ParameterProviderError, ParameterProviderResult,
};
pub use cache::CachePolicy;
#[cfg(feature = "dynamodb")]
pub use dynamodb::DynamoDbParameterProvider;
pub use parameter::{Parameter, Parameters};
pub use provider::{InMemoryParameterProvider, ParameterProvider};
#[cfg(feature = "secrets")]
pub use secrets::SecretsManagerProvider;
#[cfg(feature = "ssm")]
pub use ssm::{SsmParameterProvider, SsmParameterType, SsmParametersByName};
pub use transform::{
    ParameterTransform, ParameterTransformError, ParameterTransformErrorKind, ParameterValue,
};
