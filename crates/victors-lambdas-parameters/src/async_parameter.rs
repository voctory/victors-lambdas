//! Asynchronous parameter provider traits and facade.

use std::{
    collections::BTreeMap,
    future::Future,
    pin::Pin,
    sync::{Mutex, PoisonError},
    time::SystemTime,
};

use serde::de::DeserializeOwned;

use crate::{
    CachePolicy, InMemoryParameterProvider, Parameter, ParameterProvider, ParameterTransform,
    ParameterTransformError, ParameterValue,
};

/// Boxed future returned by asynchronous parameter providers.
pub type ParameterFuture<'a> =
    Pin<Box<dyn Future<Output = ParameterProviderResult<Option<String>>> + Send + 'a>>;

/// Result returned by asynchronous parameter providers.
pub type ParameterProviderResult<T> = Result<T, ParameterProviderError>;

/// Result returned by asynchronous parameter retrieval with optional transforms.
pub type AsyncParameterResult<T> = Result<T, AsyncParameterError>;

/// Error returned by asynchronous parameter providers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParameterProviderError {
    name: String,
    message: String,
}

impl ParameterProviderError {
    /// Creates a provider error for a parameter name.
    #[must_use]
    pub fn new(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            message: message.into(),
        }
    }

    /// Returns the parameter name associated with the provider error.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the provider error message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl std::fmt::Display for ParameterProviderError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{} provider failed: {}", self.name, self.message)
    }
}

impl std::error::Error for ParameterProviderError {}

/// Error returned by asynchronous parameter retrieval.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AsyncParameterError {
    /// The backing provider failed to retrieve a parameter.
    Provider(ParameterProviderError),
    /// The retrieved parameter value could not be transformed.
    Transform(ParameterTransformError),
}

impl std::fmt::Display for AsyncParameterError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Provider(error) => error.fmt(formatter),
            Self::Transform(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for AsyncParameterError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Provider(error) => Some(error),
            Self::Transform(error) => Some(error),
        }
    }
}

impl From<ParameterProviderError> for AsyncParameterError {
    fn from(error: ParameterProviderError) -> Self {
        Self::Provider(error)
    }
}

impl From<ParameterTransformError> for AsyncParameterError {
    fn from(error: ParameterTransformError) -> Self {
        Self::Transform(error)
    }
}

/// Retrieves parameter values asynchronously from a backing store.
pub trait AsyncParameterProvider: Sync {
    /// Gets a parameter value by name.
    fn get<'a>(&'a self, name: &'a str) -> ParameterFuture<'a>;
}

/// Asynchronous parameter retrieval facade for a provider.
#[derive(Debug)]
pub struct AsyncParameters<P> {
    provider: P,
    cache_policy: CachePolicy,
    cache: Mutex<BTreeMap<String, CachedParameter>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CachedParameter {
    value: String,
    cached_at: SystemTime,
}

impl<P> AsyncParameters<P>
where
    P: AsyncParameterProvider,
{
    /// Creates asynchronous parameter retrieval with a provider.
    #[must_use]
    pub fn new(provider: P) -> Self {
        Self::with_cache_policy(provider, CachePolicy::default())
    }

    /// Creates asynchronous parameter retrieval with a provider and cache policy.
    #[must_use]
    pub fn with_cache_policy(provider: P, cache_policy: CachePolicy) -> Self {
        Self {
            provider,
            cache_policy,
            cache: Mutex::new(BTreeMap::new()),
        }
    }

    /// Gets a parameter by name.
    ///
    /// # Errors
    ///
    /// Returns [`ParameterProviderError`] when the backing provider fails.
    pub async fn get(&self, name: &str) -> ParameterProviderResult<Option<Parameter>> {
        self.get_at(name, SystemTime::now()).await
    }

    /// Gets a parameter by name, bypassing any cached value.
    ///
    /// When the provider returns a value, the cache is updated with that value.
    /// When the provider returns no value, the cached value is removed.
    ///
    /// # Errors
    ///
    /// Returns [`ParameterProviderError`] when the backing provider fails.
    pub async fn get_force(&self, name: &str) -> ParameterProviderResult<Option<Parameter>> {
        self.fetch_at(name, SystemTime::now()).await
    }

    /// Gets a parameter and deserializes its value from JSON.
    ///
    /// # Errors
    ///
    /// Returns a transform error when the parameter exists but the value cannot
    /// be deserialized into the requested type.
    pub async fn get_json<T>(&self, name: &str) -> AsyncParameterResult<Option<T>>
    where
        T: DeserializeOwned,
    {
        self.get(name)
            .await?
            .map(|parameter| parameter.json())
            .transpose()
            .map_err(Into::into)
    }

    /// Force-fetches a parameter and deserializes its value from JSON.
    ///
    /// # Errors
    ///
    /// Returns a transform error when the parameter exists but the value cannot
    /// be deserialized into the requested type.
    pub async fn get_force_json<T>(&self, name: &str) -> AsyncParameterResult<Option<T>>
    where
        T: DeserializeOwned,
    {
        self.get_force(name)
            .await?
            .map(|parameter| parameter.json())
            .transpose()
            .map_err(Into::into)
    }

    /// Gets a parameter and decodes its value as standard base64.
    ///
    /// # Errors
    ///
    /// Returns a transform error when the parameter exists but the value is not
    /// valid base64.
    pub async fn get_binary(&self, name: &str) -> AsyncParameterResult<Option<Vec<u8>>> {
        self.get(name)
            .await?
            .map(|parameter| parameter.binary())
            .transpose()
            .map_err(Into::into)
    }

    /// Force-fetches a parameter and decodes its value as standard base64.
    ///
    /// # Errors
    ///
    /// Returns a transform error when the parameter exists but the value is not
    /// valid base64.
    pub async fn get_force_binary(&self, name: &str) -> AsyncParameterResult<Option<Vec<u8>>> {
        self.get_force(name)
            .await?
            .map(|parameter| parameter.binary())
            .transpose()
            .map_err(Into::into)
    }

    /// Gets a parameter and applies a text, JSON, binary, or auto transform.
    ///
    /// # Errors
    ///
    /// Returns a provider error when retrieval fails, or a transform error when
    /// the parameter exists but JSON or binary decoding fails.
    pub async fn get_transformed(
        &self,
        name: &str,
        transform: ParameterTransform,
    ) -> AsyncParameterResult<Option<ParameterValue>> {
        self.get(name)
            .await?
            .map(|parameter| parameter.transform(transform))
            .transpose()
            .map_err(Into::into)
    }

    /// Force-fetches a parameter and applies a text, JSON, binary, or auto transform.
    ///
    /// # Errors
    ///
    /// Returns a provider error when retrieval fails, or a transform error when
    /// the parameter exists but JSON or binary decoding fails.
    pub async fn get_force_transformed(
        &self,
        name: &str,
        transform: ParameterTransform,
    ) -> AsyncParameterResult<Option<ParameterValue>> {
        self.get_force(name)
            .await?
            .map(|parameter| parameter.transform(transform))
            .transpose()
            .map_err(Into::into)
    }

    /// Returns the provider.
    #[must_use]
    pub fn provider(&self) -> &P {
        &self.provider
    }

    /// Returns the cache policy used by this facade.
    #[must_use]
    pub const fn cache_policy(&self) -> CachePolicy {
        self.cache_policy
    }

    /// Clears all cached parameter values.
    pub fn clear_cache(&self) {
        self.cache
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clear();
    }

    /// Returns the number of cached parameter values.
    #[must_use]
    pub fn cache_len(&self) -> usize {
        self.cache
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .len()
    }

    async fn get_at(
        &self,
        name: &str,
        now: SystemTime,
    ) -> ParameterProviderResult<Option<Parameter>> {
        if let Some(value) = self.cached_value(name, now) {
            return Ok(Some(Parameter::new(name, value)));
        }

        self.fetch_at(name, now).await
    }

    async fn fetch_at(
        &self,
        name: &str,
        now: SystemTime,
    ) -> ParameterProviderResult<Option<Parameter>> {
        if let Some(value) = self.provider.get(name).await? {
            self.store_cached_value(name, &value, now);
            Ok(Some(Parameter::new(name, value)))
        } else {
            self.cache
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .remove(name);
            Ok(None)
        }
    }

    fn cached_value(&self, name: &str, now: SystemTime) -> Option<String> {
        if !self.cache_policy.enabled() {
            return None;
        }

        self.cache
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(name)
            .filter(|cached| self.cache_policy.is_fresh(cached.cached_at, now))
            .map(|cached| cached.value.clone())
    }

    fn store_cached_value(&self, name: &str, value: &str, now: SystemTime) {
        if !self.cache_policy.enabled() {
            return;
        }

        self.cache
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(
                name.to_owned(),
                CachedParameter {
                    value: value.to_owned(),
                    cached_at: now,
                },
            );
    }
}

impl<P> PartialEq for AsyncParameters<P>
where
    P: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.provider == other.provider && self.cache_policy == other.cache_policy
    }
}

impl<P> Eq for AsyncParameters<P> where P: Eq {}

impl AsyncParameterProvider for InMemoryParameterProvider {
    fn get<'a>(&'a self, name: &'a str) -> ParameterFuture<'a> {
        Box::pin(async move { Ok(ParameterProvider::get(self, name)) })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use futures_executor::block_on;
    use serde_json::{Value, json};

    use crate::{CachePolicy, ParameterTransform, ParameterTransformErrorKind, ParameterValue};

    use super::{
        AsyncParameterError, AsyncParameterProvider, AsyncParameters, InMemoryParameterProvider,
        ParameterFuture, ParameterProviderError,
    };

    #[derive(Default)]
    struct AsyncCountingProvider {
        calls: AtomicUsize,
    }

    impl AsyncParameterProvider for AsyncCountingProvider {
        fn get<'a>(&'a self, _name: &'a str) -> ParameterFuture<'a> {
            Box::pin(async move {
                let calls = self.calls.fetch_add(1, Ordering::SeqCst) + 1;
                Ok(Some(format!("value-{calls}")))
            })
        }
    }

    #[test]
    fn async_cache_reuses_cached_value() {
        let parameters = AsyncParameters::with_cache_policy(
            AsyncCountingProvider::default(),
            CachePolicy::forever(),
        );

        let first = block_on(parameters.get("name"))
            .expect("provider should succeed")
            .expect("parameter should exist");
        let second = block_on(parameters.get("name"))
            .expect("provider should succeed")
            .expect("parameter should exist");

        assert_eq!(first.value(), "value-1");
        assert_eq!(second.value(), "value-1");
        assert_eq!(parameters.provider().calls.load(Ordering::SeqCst), 1);
        assert_eq!(parameters.cache_len(), 1);
    }

    #[test]
    fn async_force_fetch_bypasses_and_updates_cache() {
        let parameters = AsyncParameters::with_cache_policy(
            AsyncCountingProvider::default(),
            CachePolicy::forever(),
        );

        let first = block_on(parameters.get("name"))
            .expect("provider should succeed")
            .expect("parameter should exist");
        let forced = block_on(parameters.get_force("name"))
            .expect("provider should succeed")
            .expect("parameter should exist");
        let cached = block_on(parameters.get("name"))
            .expect("provider should succeed")
            .expect("parameter should exist");

        assert_eq!(first.value(), "value-1");
        assert_eq!(forced.value(), "value-2");
        assert_eq!(cached.value(), "value-2");
        assert_eq!(parameters.provider().calls.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn async_transforms_parameter_values() {
        let parameters = AsyncParameters::new(
            InMemoryParameterProvider::new()
                .with_parameter("json", r#"{"enabled":true}"#)
                .with_parameter("binary", "aGVsbG8="),
        );

        let json = block_on(parameters.get_json::<Value>("json"))
            .expect("JSON transform should succeed")
            .expect("JSON parameter should exist");
        let binary = block_on(parameters.get_binary("binary"))
            .expect("binary transform should succeed")
            .expect("binary parameter should exist");

        assert_eq!(json, json!({ "enabled": true }));
        assert_eq!(binary, b"hello");
    }

    #[test]
    fn async_auto_transform_uses_parameter_name_suffix() {
        let parameters = AsyncParameters::new(
            InMemoryParameterProvider::new()
                .with_parameter("config.json", r#"{"enabled":true}"#)
                .with_parameter("key.binary", "aGVsbG8=")
                .with_parameter("plain", "hello"),
        );

        let json = block_on(parameters.get_transformed("config.json", ParameterTransform::Auto))
            .expect("auto JSON transform should succeed")
            .expect("parameter should exist");
        let binary = block_on(parameters.get_transformed("key.binary", ParameterTransform::Auto))
            .expect("auto binary transform should succeed")
            .expect("parameter should exist");
        let text = block_on(parameters.get_force_transformed("plain", ParameterTransform::Auto))
            .expect("auto text transform should succeed")
            .expect("parameter should exist");

        assert_eq!(json, ParameterValue::Json(json!({ "enabled": true })));
        assert_eq!(binary, ParameterValue::Binary(b"hello".to_vec()));
        assert_eq!(text, ParameterValue::Text("hello".to_owned()));
    }

    #[test]
    fn async_transform_errors_include_parameter_name_and_kind() {
        let parameters =
            AsyncParameters::new(InMemoryParameterProvider::new().with_parameter("json", "nope"));

        let error =
            block_on(parameters.get_json::<Value>("json")).expect_err("JSON transform should fail");
        let AsyncParameterError::Transform(error) = error else {
            panic!("expected transform error");
        };

        assert_eq!(error.name(), "json");
        assert_eq!(error.kind(), ParameterTransformErrorKind::Json);
    }

    #[test]
    fn async_provider_errors_are_returned() {
        struct FailingProvider;

        impl AsyncParameterProvider for FailingProvider {
            fn get<'a>(&'a self, name: &'a str) -> ParameterFuture<'a> {
                Box::pin(
                    async move { Err(ParameterProviderError::new(name, "transport unavailable")) },
                )
            }
        }

        let parameters = AsyncParameters::new(FailingProvider);
        let error = block_on(parameters.get("name")).expect_err("provider should fail");

        assert_eq!(error.name(), "name");
        assert_eq!(error.message(), "transport unavailable");
    }
}
