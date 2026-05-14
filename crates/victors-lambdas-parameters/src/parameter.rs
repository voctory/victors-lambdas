//! Parameter values and client facade.

use std::{collections::BTreeMap, sync::Mutex, time::SystemTime};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde::de::DeserializeOwned;

use crate::{
    CachePolicy, ParameterProvider, ParameterTransform, ParameterTransformError, ParameterValue,
};

/// A resolved parameter value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Parameter {
    name: String,
    value: String,
}

impl Parameter {
    /// Creates a parameter value.
    #[must_use]
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
        }
    }

    /// Returns the parameter name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the parameter value.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Deserializes the parameter value from JSON.
    ///
    /// # Errors
    ///
    /// Returns a transform error when the parameter value is not valid JSON for
    /// the requested type.
    pub fn json<T>(&self) -> Result<T, ParameterTransformError>
    where
        T: DeserializeOwned,
    {
        serde_json::from_str(&self.value)
            .map_err(|error| ParameterTransformError::json(self.name.clone(), error.to_string()))
    }

    /// Decodes the parameter value as standard base64.
    ///
    /// # Errors
    ///
    /// Returns a transform error when the parameter value is not valid base64.
    pub fn binary(&self) -> Result<Vec<u8>, ParameterTransformError> {
        STANDARD
            .decode(&self.value)
            .map_err(|error| ParameterTransformError::binary(self.name.clone(), error.to_string()))
    }

    /// Applies a text, JSON, binary, or auto transform to the parameter value.
    ///
    /// # Errors
    ///
    /// Returns a transform error when JSON or binary decoding fails.
    pub fn transform(
        &self,
        transform: ParameterTransform,
    ) -> Result<ParameterValue, ParameterTransformError> {
        match transform.resolve_for_name(&self.name) {
            ParameterTransform::None | ParameterTransform::Auto => {
                Ok(ParameterValue::Text(self.value.clone()))
            }
            ParameterTransform::Json => self.json().map(ParameterValue::Json),
            ParameterTransform::Binary => self.binary().map(ParameterValue::Binary),
        }
    }
}

/// Parameter retrieval facade for a provider.
#[derive(Debug)]
pub struct Parameters<P> {
    provider: P,
    cache_policy: CachePolicy,
    cache: Mutex<BTreeMap<String, CachedParameter>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CachedParameter {
    value: String,
    cached_at: SystemTime,
}

impl<P> Parameters<P>
where
    P: ParameterProvider,
{
    /// Creates parameter retrieval with a provider.
    #[must_use]
    pub fn new(provider: P) -> Self {
        Self::with_cache_policy(provider, CachePolicy::default())
    }

    /// Creates parameter retrieval with a provider and cache policy.
    #[must_use]
    pub fn with_cache_policy(provider: P, cache_policy: CachePolicy) -> Self {
        Self {
            provider,
            cache_policy,
            cache: Mutex::new(BTreeMap::new()),
        }
    }

    /// Gets a parameter by name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<Parameter> {
        self.get_at(name, SystemTime::now())
    }

    /// Gets a parameter by name, bypassing any cached value.
    ///
    /// When the provider returns a value, the cache is updated with that value.
    /// When the provider returns no value, the cached value is removed.
    #[must_use]
    pub fn get_force(&self, name: &str) -> Option<Parameter> {
        self.fetch_at(name, SystemTime::now())
    }

    /// Gets a parameter and deserializes its value from JSON.
    ///
    /// # Errors
    ///
    /// Returns a transform error when the parameter exists but the value cannot
    /// be deserialized into the requested type.
    pub fn get_json<T>(&self, name: &str) -> Result<Option<T>, ParameterTransformError>
    where
        T: DeserializeOwned,
    {
        self.get(name).map(|parameter| parameter.json()).transpose()
    }

    /// Force-fetches a parameter and deserializes its value from JSON.
    ///
    /// # Errors
    ///
    /// Returns a transform error when the parameter exists but the value cannot
    /// be deserialized into the requested type.
    pub fn get_force_json<T>(&self, name: &str) -> Result<Option<T>, ParameterTransformError>
    where
        T: DeserializeOwned,
    {
        self.get_force(name)
            .map(|parameter| parameter.json())
            .transpose()
    }

    /// Gets a parameter and decodes its value as standard base64.
    ///
    /// # Errors
    ///
    /// Returns a transform error when the parameter exists but the value is not
    /// valid base64.
    pub fn get_binary(&self, name: &str) -> Result<Option<Vec<u8>>, ParameterTransformError> {
        self.get(name)
            .map(|parameter| parameter.binary())
            .transpose()
    }

    /// Force-fetches a parameter and decodes its value as standard base64.
    ///
    /// # Errors
    ///
    /// Returns a transform error when the parameter exists but the value is not
    /// valid base64.
    pub fn get_force_binary(&self, name: &str) -> Result<Option<Vec<u8>>, ParameterTransformError> {
        self.get_force(name)
            .map(|parameter| parameter.binary())
            .transpose()
    }

    /// Gets a parameter and applies a text, JSON, binary, or auto transform.
    ///
    /// # Errors
    ///
    /// Returns a transform error when the parameter exists but JSON or binary
    /// decoding fails.
    pub fn get_transformed(
        &self,
        name: &str,
        transform: ParameterTransform,
    ) -> Result<Option<ParameterValue>, ParameterTransformError> {
        self.get(name)
            .map(|parameter| parameter.transform(transform))
            .transpose()
    }

    /// Force-fetches a parameter and applies a text, JSON, binary, or auto transform.
    ///
    /// # Errors
    ///
    /// Returns a transform error when the parameter exists but JSON or binary
    /// decoding fails.
    pub fn get_force_transformed(
        &self,
        name: &str,
        transform: ParameterTransform,
    ) -> Result<Option<ParameterValue>, ParameterTransformError> {
        self.get_force(name)
            .map(|parameter| parameter.transform(transform))
            .transpose()
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
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clear();
    }

    /// Returns the number of cached parameter values.
    #[must_use]
    pub fn cache_len(&self) -> usize {
        self.cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .len()
    }

    fn get_at(&self, name: &str, now: SystemTime) -> Option<Parameter> {
        if let Some(value) = self.cached_value(name, now) {
            return Some(Parameter::new(name, value));
        }

        self.fetch_at(name, now)
    }

    fn fetch_at(&self, name: &str, now: SystemTime) -> Option<Parameter> {
        if let Some(value) = self.provider.get(name) {
            self.store_cached_value(name, &value, now);
            Some(Parameter::new(name, value))
        } else {
            self.cache
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .remove(name);
            None
        }
    }

    fn cached_value(&self, name: &str, now: SystemTime) -> Option<String> {
        if !self.cache_policy.enabled() {
            return None;
        }

        self.cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
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
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(
                name.to_owned(),
                CachedParameter {
                    value: value.to_owned(),
                    cached_at: now,
                },
            );
    }
}

impl<P> PartialEq for Parameters<P>
where
    P: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.provider == other.provider && self.cache_policy == other.cache_policy
    }
}

impl<P> Eq for Parameters<P> where P: Eq {}

#[cfg(test)]
mod tests {
    use std::{
        cell::Cell,
        time::{Duration, UNIX_EPOCH},
    };

    use serde_json::{Value, json};

    use crate::{
        CachePolicy, ParameterProvider, ParameterTransform, ParameterTransformErrorKind,
        ParameterValue,
    };

    use super::{Parameter, Parameters};

    #[derive(Default)]
    struct CountingProvider {
        calls: Cell<usize>,
    }

    impl ParameterProvider for CountingProvider {
        fn get(&self, _name: &str) -> Option<String> {
            let calls = self.calls.get() + 1;
            self.calls.set(calls);
            Some(format!("value-{calls}"))
        }
    }

    #[test]
    fn disabled_cache_fetches_each_time() {
        let parameters = Parameters::new(CountingProvider::default());

        assert_eq!(
            parameters.get("name").as_ref().map(Parameter::value),
            Some("value-1")
        );
        assert_eq!(
            parameters.get("name").as_ref().map(Parameter::value),
            Some("value-2")
        );
        assert_eq!(parameters.provider().calls.get(), 2);
        assert_eq!(parameters.cache_len(), 0);
    }

    #[test]
    fn forever_cache_reuses_cached_value() {
        let parameters =
            Parameters::with_cache_policy(CountingProvider::default(), CachePolicy::forever());

        assert_eq!(
            parameters.get("name").as_ref().map(Parameter::value),
            Some("value-1")
        );
        assert_eq!(
            parameters.get("name").as_ref().map(Parameter::value),
            Some("value-1")
        );
        assert_eq!(parameters.provider().calls.get(), 1);
        assert_eq!(parameters.cache_len(), 1);
    }

    #[test]
    fn clear_cache_forces_next_fetch() {
        let parameters =
            Parameters::with_cache_policy(CountingProvider::default(), CachePolicy::forever());

        assert_eq!(
            parameters.get("name").as_ref().map(Parameter::value),
            Some("value-1")
        );
        parameters.clear_cache();
        assert_eq!(
            parameters.get("name").as_ref().map(Parameter::value),
            Some("value-2")
        );
    }

    #[test]
    fn force_fetch_bypasses_and_updates_cache() {
        let parameters =
            Parameters::with_cache_policy(CountingProvider::default(), CachePolicy::forever());

        assert_eq!(
            parameters.get("name").as_ref().map(Parameter::value),
            Some("value-1")
        );
        assert_eq!(
            parameters.get("name").as_ref().map(Parameter::value),
            Some("value-1")
        );
        assert_eq!(
            parameters.get_force("name").as_ref().map(Parameter::value),
            Some("value-2")
        );
        assert_eq!(
            parameters.get("name").as_ref().map(Parameter::value),
            Some("value-2")
        );
        assert_eq!(parameters.provider().calls.get(), 2);
    }

    #[test]
    fn json_transform_deserializes_parameter_values() {
        let parameters = Parameters::new(
            crate::InMemoryParameterProvider::new()
                .with_parameter("/service/config", r#"{"retries":3}"#),
        );

        let value = parameters
            .get_json::<Value>("/service/config")
            .expect("json transform should succeed")
            .expect("parameter should exist");

        assert_eq!(value, json!({ "retries": 3 }));
    }

    #[test]
    fn binary_transform_decodes_base64_parameter_values() {
        let parameters = Parameters::new(
            crate::InMemoryParameterProvider::new().with_parameter("/service/key", "c2VjcmV0"),
        );

        let value = parameters
            .get_binary("/service/key")
            .expect("binary transform should succeed")
            .expect("parameter should exist");

        assert_eq!(value, b"secret");
    }

    #[test]
    fn transform_api_returns_text_json_or_binary_values() {
        let parameters = Parameters::new(
            crate::InMemoryParameterProvider::new()
                .with_parameter("/service/plain", "text")
                .with_parameter("/service/config", r#"{"retries":3}"#)
                .with_parameter("/service/key", "c2VjcmV0"),
        );

        let text = parameters
            .get_transformed("/service/plain", ParameterTransform::None)
            .expect("text transform should succeed")
            .expect("parameter should exist");
        let json = parameters
            .get_transformed("/service/config", ParameterTransform::Json)
            .expect("json transform should succeed")
            .expect("parameter should exist");
        let binary = parameters
            .get_transformed("/service/key", ParameterTransform::Binary)
            .expect("binary transform should succeed")
            .expect("parameter should exist");

        assert_eq!(text.as_text(), Some("text"));
        assert_eq!(json.as_json(), Some(&json!({ "retries": 3 })));
        assert_eq!(binary.as_binary(), Some(b"secret".as_slice()));
    }

    #[test]
    fn auto_transform_uses_parameter_name_suffix() {
        let parameters = Parameters::new(
            crate::InMemoryParameterProvider::new()
                .with_parameter("/service/config.JSON", r#"{"enabled":true}"#)
                .with_parameter("/service/key.binary", "aGVsbG8=")
                .with_parameter("/service/plain", "hello"),
        );

        let json = parameters
            .get_transformed("/service/config.JSON", ParameterTransform::Auto)
            .expect("auto JSON transform should succeed")
            .expect("parameter should exist");
        let binary = parameters
            .get_transformed("/service/key.binary", ParameterTransform::Auto)
            .expect("auto binary transform should succeed")
            .expect("parameter should exist");
        let text = parameters
            .get_force_transformed("/service/plain", ParameterTransform::Auto)
            .expect("auto text transform should succeed")
            .expect("parameter should exist");

        assert_eq!(json, ParameterValue::Json(json!({ "enabled": true })));
        assert_eq!(binary, ParameterValue::Binary(b"hello".to_vec()));
        assert_eq!(text, ParameterValue::Text("hello".to_owned()));
    }

    #[test]
    fn transform_errors_include_parameter_name_and_kind() {
        let parameter = Parameter::new("/service/config", "{");

        let error = parameter
            .json::<Value>()
            .expect_err("invalid json should fail");

        assert_eq!(error.kind(), ParameterTransformErrorKind::Json);
        assert_eq!(error.name(), "/service/config");
        assert!(error.message().contains("EOF"));
    }

    #[test]
    fn ttl_cache_reuses_fresh_values_and_refetches_expired_values() {
        let parameters = Parameters::with_cache_policy(
            CountingProvider::default(),
            CachePolicy::ttl(Duration::from_secs(5)),
        );

        assert_eq!(
            parameters
                .get_at("name", UNIX_EPOCH)
                .as_ref()
                .map(Parameter::value),
            Some("value-1")
        );
        assert_eq!(
            parameters
                .get_at("name", UNIX_EPOCH + Duration::from_secs(4))
                .as_ref()
                .map(Parameter::value),
            Some("value-1")
        );
        assert_eq!(
            parameters
                .get_at("name", UNIX_EPOCH + Duration::from_secs(5))
                .as_ref()
                .map(Parameter::value),
            Some("value-2")
        );
        assert_eq!(parameters.provider().calls.get(), 2);
    }
}
