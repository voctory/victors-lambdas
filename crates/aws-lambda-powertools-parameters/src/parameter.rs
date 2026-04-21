//! Parameter values and client facade.

use std::{collections::BTreeMap, sync::Mutex, time::SystemTime};

use crate::{CachePolicy, ParameterProvider};

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

    use crate::{CachePolicy, ParameterProvider};

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
