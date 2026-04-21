//! Parameter provider traits.

/// Retrieves parameter values from a backing store.
pub trait ParameterProvider {
    /// Gets a parameter value by name.
    fn get(&self, name: &str) -> Option<String>;
}
