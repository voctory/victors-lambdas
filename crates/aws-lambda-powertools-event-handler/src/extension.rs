//! Typed extension storage for event-handler requests and routers.

use std::{
    any::{Any, TypeId},
    fmt,
    sync::Arc,
};

type ExtensionValue = Arc<dyn Any + Send + Sync>;

/// Type-indexed extension values.
///
/// Extensions let middleware attach typed data that later middleware, route
/// handlers, response middleware, and error handlers can read without using
/// stringly typed maps. One value can be stored for each concrete type.
#[derive(Clone, Default)]
pub struct Extensions {
    values: Vec<(TypeId, ExtensionValue)>,
}

impl Extensions {
    /// Creates an empty extension store.
    #[must_use]
    pub const fn new() -> Self {
        Self { values: Vec::new() }
    }

    /// Inserts a typed value, replacing any previous value with the same type.
    pub fn insert<T>(&mut self, value: T) -> &mut Self
    where
        T: Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();
        self.values
            .retain(|(existing_type_id, _)| *existing_type_id != type_id);
        self.values.push((type_id, Arc::new(value)));
        self
    }

    /// Returns a copy of this store with a typed value inserted.
    #[must_use]
    pub fn with<T>(mut self, value: T) -> Self
    where
        T: Send + Sync + 'static,
    {
        self.insert(value);
        self
    }

    /// Returns a typed value when present.
    #[must_use]
    pub fn get<T>(&self) -> Option<&T>
    where
        T: Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();
        self.values.iter().find_map(|(existing_type_id, value)| {
            (*existing_type_id == type_id).then(|| value.downcast_ref::<T>())?
        })
    }

    /// Returns true when a value of this type is present.
    #[must_use]
    pub fn contains<T>(&self) -> bool
    where
        T: Send + Sync + 'static,
    {
        self.get::<T>().is_some()
    }

    /// Removes and returns a typed value when present.
    pub fn remove<T>(&mut self) -> Option<Arc<T>>
    where
        T: Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();
        let position = self
            .values
            .iter()
            .position(|(existing_type_id, _)| *existing_type_id == type_id)?;
        let (_, value) = self.values.remove(position);

        Arc::downcast::<T>(value).ok()
    }

    /// Removes all extension values.
    pub fn clear(&mut self) -> &mut Self {
        self.values.clear();
        self
    }

    /// Returns the number of stored extension values.
    #[must_use]
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Returns true when no extension values are stored.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub(crate) fn extend_missing(&mut self, other: Self) {
        for (type_id, value) in other.values {
            if !self
                .values
                .iter()
                .any(|(existing_type_id, _)| *existing_type_id == type_id)
            {
                self.values.push((type_id, value));
            }
        }
    }
}

impl fmt::Debug for Extensions {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Extensions")
            .field("len", &self.values.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::Extensions;

    #[derive(Debug, Eq, PartialEq)]
    struct CorrelationId(&'static str);

    #[test]
    fn extensions_store_values_by_type() {
        let mut extensions = Extensions::new().with(CorrelationId("request-1"));
        extensions.insert(42_u16);

        assert_eq!(
            extensions.get::<CorrelationId>(),
            Some(&CorrelationId("request-1"))
        );
        assert_eq!(extensions.get::<u16>(), Some(&42));
        assert!(extensions.contains::<CorrelationId>());
        assert_eq!(extensions.len(), 2);
    }

    #[test]
    fn extensions_replace_and_remove_values_by_type() {
        let mut extensions = Extensions::new().with(CorrelationId("request-1"));
        extensions.insert(CorrelationId("request-2"));

        let removed = extensions
            .remove::<CorrelationId>()
            .expect("correlation id is present");

        assert_eq!(*removed, CorrelationId("request-2"));
        assert!(extensions.is_empty());
    }

    #[test]
    fn extend_missing_keeps_existing_values() {
        let mut extensions = Extensions::new()
            .with(CorrelationId("request-1"))
            .with(7_u8);
        let other = Extensions::new()
            .with(CorrelationId("request-2"))
            .with("checkout");

        extensions.extend_missing(other);

        assert_eq!(
            extensions.get::<CorrelationId>(),
            Some(&CorrelationId("request-1"))
        );
        assert_eq!(extensions.get::<u8>(), Some(&7));
        assert_eq!(extensions.get::<&'static str>(), Some(&"checkout"));
    }
}
