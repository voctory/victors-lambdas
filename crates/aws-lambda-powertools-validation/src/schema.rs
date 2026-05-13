//! JSON Schema validator cache.

use std::collections::BTreeMap;

use crate::{ValidationError, ValidationResult};

/// Caches compiled JSON Schema validators by caller-provided name.
///
/// Compiling schemas can be more expensive than validating payloads. This cache
/// lets Lambda handlers compile once during initialization or first use and then
/// validate repeated payloads against the compiled validator.
#[derive(Clone, Debug, Default)]
pub struct JsonSchemaCache {
    schemas: BTreeMap<String, jsonschema::Validator>,
}

impl JsonSchemaCache {
    /// Creates an empty schema cache.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            schemas: BTreeMap::new(),
        }
    }

    /// Compiles and stores a schema under `name`.
    ///
    /// If a schema already exists for the same name, it is replaced after the
    /// new schema compiles successfully.
    ///
    /// # Errors
    ///
    /// Returns a validation error when `schema` is not a valid JSON Schema.
    pub fn compile_schema(
        &mut self,
        name: impl Into<String>,
        schema: &serde_json::Value,
    ) -> ValidationResult {
        let validator = compile(schema)?;

        self.schemas.insert(name.into(), validator);
        Ok(())
    }

    /// Validates an instance against a cached schema.
    ///
    /// # Errors
    ///
    /// Returns a validation error when `name` is not cached or when `instance`
    /// does not satisfy the cached schema.
    pub fn validate(&self, name: &str, instance: &serde_json::Value) -> ValidationResult {
        let validator = self.schemas.get(name).ok_or_else(|| missing_schema(name))?;

        validator
            .validate(instance)
            .map_err(|error| ValidationError::json_schema(error.to_string()))
    }

    /// Validates an instance, compiling and caching `schema` first when needed.
    ///
    /// # Errors
    ///
    /// Returns a validation error when the schema must be compiled and is
    /// invalid, or when `instance` does not satisfy the cached schema.
    pub fn validate_or_compile(
        &mut self,
        name: impl Into<String>,
        schema: &serde_json::Value,
        instance: &serde_json::Value,
    ) -> ValidationResult {
        let name = name.into();

        if !self.schemas.contains_key(&name) {
            self.compile_schema(name.clone(), schema)?;
        }

        self.validate(&name, instance)
    }

    /// Returns true when a schema is cached under `name`.
    #[must_use]
    pub fn contains(&self, name: &str) -> bool {
        self.schemas.contains_key(name)
    }

    /// Returns the number of cached schemas.
    #[must_use]
    pub fn len(&self) -> usize {
        self.schemas.len()
    }

    /// Returns true when the cache contains no schemas.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.schemas.is_empty()
    }

    /// Removes a cached schema.
    ///
    /// Returns true when a schema existed for `name`.
    pub fn remove(&mut self, name: &str) -> bool {
        self.schemas.remove(name).is_some()
    }

    /// Removes all cached schemas.
    pub fn clear(&mut self) {
        self.schemas.clear();
    }
}

fn compile(schema: &serde_json::Value) -> Result<jsonschema::Validator, ValidationError> {
    jsonschema::validator_for(schema)
        .map_err(|error| ValidationError::invalid("schema", error.to_string()))
}

fn missing_schema(name: &str) -> ValidationError {
    ValidationError::invalid("schema", format!("schema {name} is not cached"))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::JsonSchemaCache;
    use crate::ValidationErrorKind;

    #[test]
    fn validates_against_cached_schema() {
        let schema = json!({
            "type": "object",
            "required": ["order_id", "quantity"],
            "properties": {
                "order_id": { "type": "string" },
                "quantity": { "type": "integer", "minimum": 1 }
            }
        });
        let valid = json!({
            "order_id": "order-1",
            "quantity": 2
        });
        let invalid = json!({
            "order_id": "order-1",
            "quantity": 0
        });
        let mut cache = JsonSchemaCache::new();

        cache
            .compile_schema("order", &schema)
            .expect("schema should compile");

        assert!(cache.contains("order"));
        assert_eq!(cache.len(), 1);
        assert!(cache.validate("order", &valid).is_ok());

        let error = cache
            .validate("order", &invalid)
            .expect_err("invalid payload should fail");

        assert_eq!(error.kind(), ValidationErrorKind::Schema);
        assert!(error.message().contains("minimum"));
    }

    #[test]
    fn validate_or_compile_reuses_cached_schema() {
        let schema = json!({
            "type": "object",
            "required": ["name"],
            "properties": {
                "name": { "type": "string" }
            }
        });
        let valid = json!({ "name": "newsletter" });
        let invalid = json!({});
        let mut cache = JsonSchemaCache::new();

        cache
            .validate_or_compile("subscription", &schema, &valid)
            .expect("valid payload should pass");
        cache
            .validate_or_compile("subscription", &json!(false), &valid)
            .expect("cached schema should be reused");

        assert_eq!(cache.len(), 1);

        let error = cache
            .validate_or_compile("subscription", &schema, &invalid)
            .expect_err("cached schema should reject invalid payload");

        assert_eq!(error.kind(), ValidationErrorKind::Schema);
    }

    #[test]
    fn missing_schema_returns_invalid_error() {
        let error = JsonSchemaCache::new()
            .validate("missing", &json!({}))
            .expect_err("missing schema should fail");

        assert_eq!(error.kind(), ValidationErrorKind::Invalid);
        assert_eq!(error.field(), Some("schema"));
        assert_eq!(error.message(), "schema missing is not cached");
    }

    #[test]
    fn invalid_schema_is_not_cached() {
        let mut cache = JsonSchemaCache::new();

        let error = cache
            .compile_schema("broken", &json!({ "type": 1 }))
            .expect_err("invalid schema should fail");

        assert_eq!(error.kind(), ValidationErrorKind::Invalid);
        assert!(cache.is_empty());
    }

    #[test]
    fn remove_and_clear_update_cache_size() {
        let mut cache = JsonSchemaCache::new();

        cache
            .compile_schema("first", &json!({ "type": "object" }))
            .expect("schema should compile");
        cache
            .compile_schema("second", &json!({ "type": "string" }))
            .expect("schema should compile");

        assert!(cache.remove("first"));
        assert!(!cache.remove("first"));
        assert_eq!(cache.len(), 1);

        cache.clear();

        assert!(cache.is_empty());
    }
}
