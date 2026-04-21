//! Parameter values and client facade.

use crate::ParameterProvider;

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
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Parameters<P> {
    provider: P,
}

impl<P> Parameters<P>
where
    P: ParameterProvider,
{
    /// Creates parameter retrieval with a provider.
    #[must_use]
    pub fn new(provider: P) -> Self {
        Self { provider }
    }

    /// Gets a parameter by name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<Parameter> {
        self.provider
            .get(name)
            .map(|value| Parameter::new(name, value))
    }

    /// Returns the provider.
    #[must_use]
    pub fn provider(&self) -> &P {
        &self.provider
    }
}
