//! Parameter provider traits.

use std::collections::BTreeMap;

/// Retrieves parameter values from a backing store.
pub trait ParameterProvider {
    /// Gets a parameter value by name.
    fn get(&self, name: &str) -> Option<String>;
}

/// In-memory parameter provider for tests and local examples.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct InMemoryParameterProvider {
    parameters: BTreeMap<String, String>,
}

impl InMemoryParameterProvider {
    /// Creates an empty in-memory parameter provider.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates an in-memory parameter provider from name/value pairs.
    #[must_use]
    pub fn from_pairs<I, K, V>(parameters: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        parameters.into_iter().collect()
    }

    /// Adds a parameter and returns the provider.
    #[must_use]
    pub fn with_parameter(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.insert(name, value);
        self
    }

    /// Inserts or replaces a parameter value.
    pub fn insert(&mut self, name: impl Into<String>, value: impl Into<String>) -> Option<String> {
        self.parameters.insert(name.into(), value.into())
    }

    /// Removes a parameter value.
    pub fn remove(&mut self, name: &str) -> Option<String> {
        self.parameters.remove(name)
    }

    /// Returns whether a parameter exists.
    #[must_use]
    pub fn contains(&self, name: &str) -> bool {
        self.parameters.contains_key(name)
    }

    /// Returns the number of stored parameters.
    #[must_use]
    pub fn len(&self) -> usize {
        self.parameters.len()
    }

    /// Returns whether no parameters are stored.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.parameters.is_empty()
    }

    /// Iterates over stored parameter names and values.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.parameters
            .iter()
            .map(|(name, value)| (name.as_str(), value.as_str()))
    }
}

impl<K, V> FromIterator<(K, V)> for InMemoryParameterProvider
where
    K: Into<String>,
    V: Into<String>,
{
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        let mut provider = Self::new();
        for (name, value) in iter {
            provider.insert(name, value);
        }
        provider
    }
}

impl ParameterProvider for InMemoryParameterProvider {
    fn get(&self, name: &str) -> Option<String> {
        self.parameters.get(name).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::{InMemoryParameterProvider, ParameterProvider};

    #[test]
    fn in_memory_provider_returns_inserted_values() {
        let provider = InMemoryParameterProvider::new()
            .with_parameter("/service/name", "checkout")
            .with_parameter("/service/retries", "3");

        assert_eq!(provider.get("/service/name").as_deref(), Some("checkout"));
        assert_eq!(provider.get("/service/retries").as_deref(), Some("3"));
        assert_eq!(provider.get("/service/missing"), None);
    }

    #[test]
    fn in_memory_provider_replaces_and_removes_values() {
        let mut provider = InMemoryParameterProvider::new();

        assert!(provider.is_empty());
        assert_eq!(provider.insert("name", "old"), None);
        assert_eq!(provider.insert("name", "new").as_deref(), Some("old"));
        assert!(provider.contains("name"));
        assert_eq!(provider.len(), 1);
        assert_eq!(provider.remove("name").as_deref(), Some("new"));
        assert!(!provider.contains("name"));
    }

    #[test]
    fn in_memory_provider_collects_from_pairs() {
        let provider = InMemoryParameterProvider::from_pairs([("one", "1"), ("two", "2")]);
        let names = provider.iter().map(|(name, _)| name).collect::<Vec<_>>();

        assert_eq!(names, vec!["one", "two"]);
    }
}
