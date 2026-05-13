//! Feature flag evaluator.

use std::cmp::Ordering;

use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Map, Value};

use crate::{
    FeatureFlag, FeatureFlagConfig, FeatureFlagError, FeatureFlagResult, FeatureFlagStore,
    FeatureRule, RuleAction,
};

/// Context values used while evaluating dynamic feature flag rules.
pub type FeatureFlagContext = Map<String, Value>;

/// Evaluates feature flags against a configured store.
#[derive(Clone, Debug, PartialEq)]
pub struct FeatureFlags<S> {
    store: S,
}

impl<S> FeatureFlags<S> {
    /// Creates a feature flag evaluator with a store provider.
    #[must_use]
    pub const fn new(store: S) -> Self {
        Self { store }
    }

    /// Returns the configured store provider.
    #[must_use]
    pub const fn store(&self) -> &S {
        &self.store
    }
}

impl<S> FeatureFlags<S>
where
    S: FeatureFlagStore,
{
    /// Gets the validated feature flag configuration from the configured store.
    ///
    /// # Errors
    ///
    /// Returns a store or configuration error when the store cannot provide a
    /// feature flag configuration.
    pub fn get_configuration(&self) -> FeatureFlagResult<FeatureFlagConfig> {
        self.store.get_configuration()
    }

    /// Evaluates a feature and returns its JSON value.
    ///
    /// If the store cannot load configuration or the feature does not exist,
    /// this returns the provided default value.
    #[must_use]
    pub fn evaluate_value(
        &self,
        name: &str,
        context: &FeatureFlagContext,
        default: Value,
    ) -> Value {
        let Ok(config) = self.store.get_configuration() else {
            return default;
        };

        config
            .get(name)
            .map_or(default, |feature| evaluate_feature(feature, context))
    }

    /// Evaluates a boolean feature.
    ///
    /// If the store cannot load configuration or the feature does not exist,
    /// this returns the provided default value.
    ///
    /// # Errors
    ///
    /// Returns a transform error when the matched feature value is not a JSON
    /// boolean.
    pub fn evaluate_bool(
        &self,
        name: &str,
        context: &FeatureFlagContext,
        default: bool,
    ) -> FeatureFlagResult<bool> {
        let value = self.evaluate_value(name, context, Value::Bool(default));
        value.as_bool().ok_or_else(|| {
            FeatureFlagError::transform(name, format!("feature value is not a boolean: {value}"))
        })
    }

    /// Evaluates a feature and deserializes the JSON value into the requested type.
    ///
    /// If the store cannot load configuration or the feature does not exist,
    /// this deserializes the provided default value.
    ///
    /// # Errors
    ///
    /// Returns a transform error when the default cannot be serialized or the
    /// evaluated feature value cannot be deserialized into the requested type.
    pub fn evaluate_json<T>(
        &self,
        name: &str,
        context: &FeatureFlagContext,
        default: T,
    ) -> FeatureFlagResult<T>
    where
        T: DeserializeOwned + Serialize,
    {
        let default = serde_json::to_value(default).map_err(|error| {
            FeatureFlagError::transform(name, format!("default value is not JSON: {error}"))
        })?;
        let value = self.evaluate_value(name, context, default);

        serde_json::from_value(value).map_err(|error| {
            FeatureFlagError::transform(
                name,
                format!("feature value has unexpected shape: {error}"),
            )
        })
    }

    /// Returns boolean features enabled for the provided context.
    ///
    /// Store failures return an empty list.
    #[must_use]
    pub fn get_enabled_features(&self, context: &FeatureFlagContext) -> Vec<String> {
        let Ok(config) = self.store.get_configuration() else {
            return Vec::new();
        };

        config
            .iter()
            .filter_map(|(name, feature)| {
                evaluate_feature(feature, context)
                    .as_bool()
                    .filter(|enabled| *enabled)
                    .map(|_| name.to_owned())
            })
            .collect()
    }
}

fn evaluate_feature(feature: &FeatureFlag, context: &FeatureFlagContext) -> Value {
    if !feature.has_rules() {
        return feature.default_value().clone();
    }

    feature
        .rules()
        .find_map(|(_, rule)| evaluate_rule(rule, context).then(|| rule.when_match().clone()))
        .unwrap_or_else(|| feature.default_value().clone())
}

fn evaluate_rule(rule: &FeatureRule, context: &FeatureFlagContext) -> bool {
    !rule.is_empty()
        && rule
            .conditions()
            .all(|condition| match context.get(condition.key()) {
                Some(context_value) => {
                    matches_condition(condition.action(), context_value, condition.value())
                }
                None => false,
            })
}

fn matches_condition(action: RuleAction, context_value: &Value, condition_value: &Value) -> bool {
    match action {
        RuleAction::Equals => context_value == condition_value,
        RuleAction::NotEquals => context_value != condition_value,
        RuleAction::KeyGreaterThanValue => compare_values(context_value, condition_value)
            .is_some_and(|ordering| ordering == Ordering::Greater),
        RuleAction::KeyGreaterThanOrEqualValue => compare_values(context_value, condition_value)
            .is_some_and(|ordering| ordering == Ordering::Greater || ordering == Ordering::Equal),
        RuleAction::KeyLessThanValue => compare_values(context_value, condition_value)
            .is_some_and(|ordering| ordering == Ordering::Less),
        RuleAction::KeyLessThanOrEqualValue => compare_values(context_value, condition_value)
            .is_some_and(|ordering| ordering == Ordering::Less || ordering == Ordering::Equal),
        RuleAction::StartsWith => string_values(context_value, condition_value)
            .is_some_and(|(context, condition)| context.starts_with(condition)),
        RuleAction::EndsWith => string_values(context_value, condition_value)
            .is_some_and(|(context, condition)| context.ends_with(condition)),
        RuleAction::In | RuleAction::KeyInValue => contains_value(condition_value, context_value),
        RuleAction::NotIn | RuleAction::KeyNotInValue => {
            !contains_value(condition_value, context_value)
        }
        RuleAction::ValueInKey => contains_value(context_value, condition_value),
        RuleAction::ValueNotInKey => !contains_value(context_value, condition_value),
        RuleAction::AllInValue => {
            compare_collection(context_value, condition_value, CollectionMatch::All)
        }
        RuleAction::AnyInValue => {
            compare_collection(context_value, condition_value, CollectionMatch::Any)
        }
        RuleAction::NoneInValue => {
            compare_collection(context_value, condition_value, CollectionMatch::None)
        }
        RuleAction::ModuloRange => compare_modulo_range(context_value, condition_value),
    }
}

fn compare_values(left: &Value, right: &Value) -> Option<Ordering> {
    match (left, right) {
        (Value::Number(left), Value::Number(right)) => left.as_f64()?.partial_cmp(&right.as_f64()?),
        (Value::String(left), Value::String(right)) => Some(left.cmp(right)),
        _ => None,
    }
}

fn string_values<'a>(left: &'a Value, right: &'a Value) -> Option<(&'a str, &'a str)> {
    Some((left.as_str()?, right.as_str()?))
}

fn contains_value(container: &Value, needle: &Value) -> bool {
    match container {
        Value::Array(values) => values.iter().any(|value| value == needle),
        Value::Object(values) => needle.as_str().is_some_and(|key| values.contains_key(key)),
        Value::String(value) => needle.as_str().is_some_and(|needle| value.contains(needle)),
        _ => false,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CollectionMatch {
    All,
    Any,
    None,
}

fn compare_collection(
    context_value: &Value,
    condition_value: &Value,
    collection_match: CollectionMatch,
) -> bool {
    let (Some(context_values), Some(condition_values)) =
        (context_value.as_array(), condition_value.as_array())
    else {
        return false;
    };

    match collection_match {
        CollectionMatch::All => context_values
            .iter()
            .all(|value| condition_values.contains(value)),
        CollectionMatch::Any => context_values
            .iter()
            .any(|value| condition_values.contains(value)),
        CollectionMatch::None => context_values
            .iter()
            .all(|value| !condition_values.contains(value)),
    }
}

fn compare_modulo_range(context_value: &Value, condition_value: &Value) -> bool {
    let Some(context_value) = integer_value(context_value) else {
        return false;
    };
    let Some(condition) = condition_value.as_object() else {
        return false;
    };

    let Some(base) = condition_integer(condition, "BASE").filter(|base| *base > 0) else {
        return false;
    };
    let Some(start) = condition_integer(condition, "START") else {
        return false;
    };
    let Some(end) = condition_integer(condition, "END") else {
        return false;
    };

    let remainder = context_value % base;
    start <= remainder && remainder <= end
}

fn condition_integer(condition: &Map<String, Value>, key: &str) -> Option<i128> {
    condition.get(key).and_then(integer_value)
}

fn integer_value(value: &Value) -> Option<i128> {
    value
        .as_i64()
        .map(i128::from)
        .or_else(|| value.as_u64().map(i128::from))
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};
    use serde_json::{Value, json};

    use super::{FeatureFlagContext, FeatureFlags};
    use crate::{
        FeatureCondition, FeatureFlag, FeatureFlagConfig, FeatureFlagError, FeatureFlagErrorKind,
        FeatureFlagResult, FeatureFlagStore, FeatureRule, InMemoryFeatureFlagStore, RuleAction,
    };

    fn context(pairs: impl IntoIterator<Item = (&'static str, Value)>) -> FeatureFlagContext {
        pairs
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect()
    }

    #[test]
    fn evaluates_static_boolean_feature() {
        let flags = FeatureFlags::new(InMemoryFeatureFlagStore::from_config(
            FeatureFlagConfig::new()
                .with_feature("ten_percent_off_campaign", FeatureFlag::boolean(true)),
        ));

        assert!(
            flags
                .evaluate_bool(
                    "ten_percent_off_campaign",
                    &FeatureFlagContext::new(),
                    false,
                )
                .unwrap()
        );
    }

    #[test]
    fn evaluates_first_matching_rule() {
        let feature = FeatureFlag::boolean(false)
            .with_rule(
                "standard tier",
                FeatureRule::new(
                    false,
                    [FeatureCondition::new(
                        RuleAction::Equals,
                        "tier",
                        "standard",
                    )],
                ),
            )
            .with_rule(
                "premium tier",
                FeatureRule::new(
                    true,
                    [FeatureCondition::new(RuleAction::Equals, "tier", "premium")],
                ),
            );
        let flags = FeatureFlags::new(InMemoryFeatureFlagStore::from_config(
            FeatureFlagConfig::new().with_feature("premium_features", feature),
        ));

        let context = context([("tier", json!("premium"))]);

        assert!(
            flags
                .evaluate_bool("premium_features", &context, false)
                .unwrap()
        );
    }

    #[test]
    fn returns_default_for_missing_feature() {
        let flags = FeatureFlags::new(InMemoryFeatureFlagStore::new());

        assert!(
            flags
                .evaluate_bool("missing_feature", &FeatureFlagContext::new(), true)
                .unwrap()
        );
    }

    #[test]
    fn returns_enabled_boolean_features() {
        let premium = FeatureFlag::boolean(false).with_rule(
            "premium tier",
            FeatureRule::new(
                true,
                [FeatureCondition::new(RuleAction::Equals, "tier", "premium")],
            ),
        );
        let flags = FeatureFlags::new(InMemoryFeatureFlagStore::from_config(
            FeatureFlagConfig::new()
                .with_feature("premium_features", premium)
                .with_feature("ten_percent_off_campaign", FeatureFlag::boolean(true))
                .with_feature(
                    "json_payload",
                    FeatureFlag::value(json!({"group": "read-only"})),
                ),
        ));
        let context = context([("tier", json!("premium"))]);

        assert_eq!(
            flags.get_enabled_features(&context),
            vec!["premium_features", "ten_percent_off_campaign"]
        );
    }

    #[test]
    fn evaluates_json_value_features() {
        #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
        struct Permissions {
            group: String,
        }

        let feature = FeatureFlag::value(json!({"group": "read-only"})).with_rule(
            "admin tenant",
            FeatureRule::new(
                json!({"group": "admin"}),
                [FeatureCondition::new(
                    RuleAction::KeyInValue,
                    "tenant_id",
                    json!(["tenant-1", "tenant-2"]),
                )],
            ),
        );
        let flags = FeatureFlags::new(InMemoryFeatureFlagStore::from_config(
            FeatureFlagConfig::new().with_feature("permissions", feature),
        ));
        let context = context([("tenant_id", json!("tenant-2"))]);

        let permissions: Permissions = flags
            .evaluate_json(
                "permissions",
                &context,
                Permissions {
                    group: "none".to_owned(),
                },
            )
            .unwrap();

        assert_eq!(
            permissions,
            Permissions {
                group: "admin".to_owned()
            }
        );
    }

    #[test]
    fn supports_string_and_collection_comparisons() {
        let feature = FeatureFlag::boolean(false)
            .with_rule(
                "email suffix",
                FeatureRule::new(
                    true,
                    [FeatureCondition::new(
                        RuleAction::EndsWith,
                        "email",
                        "@example.com",
                    )],
                ),
            )
            .with_rule(
                "group contains beta",
                FeatureRule::new(
                    true,
                    [FeatureCondition::new(
                        RuleAction::AnyInValue,
                        "groups",
                        json!(["beta", "admin"]),
                    )],
                ),
            );
        let flags = FeatureFlags::new(InMemoryFeatureFlagStore::from_config(
            FeatureFlagConfig::new().with_feature("beta", feature),
        ));

        assert!(
            flags
                .evaluate_bool(
                    "beta",
                    &context([("email", json!("person@example.com"))]),
                    false,
                )
                .unwrap()
        );
        assert!(
            flags
                .evaluate_bool(
                    "beta",
                    &context([("groups", json!(["standard", "beta"]))]),
                    false,
                )
                .unwrap()
        );
    }

    #[test]
    fn supports_numeric_comparison_and_modulo_range() {
        let feature = FeatureFlag::boolean(false).with_rule(
            "experiment segment",
            FeatureRule::new(
                true,
                [
                    FeatureCondition::new(RuleAction::KeyGreaterThanOrEqualValue, "age", 18),
                    FeatureCondition::new(
                        RuleAction::ModuloRange,
                        "user_id",
                        json!({"BASE": 100, "START": 0, "END": 19}),
                    ),
                ],
            ),
        );
        let flags = FeatureFlags::new(InMemoryFeatureFlagStore::from_config(
            FeatureFlagConfig::new().with_feature("sale_experiment", feature),
        ));
        let context = context([("age", json!(42)), ("user_id", json!(119))]);

        assert!(
            flags
                .evaluate_bool("sale_experiment", &context, false)
                .unwrap()
        );
    }

    #[test]
    fn parses_python_compatible_schema() {
        let store = InMemoryFeatureFlagStore::from_json_str(
            r#"
            {
                "premium_features": {
                    "default": false,
                    "rules": {
                        "customer tier equals premium": {
                            "when_match": true,
                            "conditions": [
                                {
                                    "action": "EQUALS",
                                    "key": "tier",
                                    "value": "premium"
                                }
                            ]
                        }
                    }
                },
                "ten_percent_off_campaign": {
                    "default": true
                }
            }
            "#,
        )
        .unwrap();
        let flags = FeatureFlags::new(store);

        assert_eq!(
            flags.get_enabled_features(&context([("tier", json!("premium"))])),
            vec!["premium_features", "ten_percent_off_campaign"]
        );
    }

    #[test]
    fn reports_non_boolean_feature_when_bool_requested() {
        let flags = FeatureFlags::new(InMemoryFeatureFlagStore::from_config(
            FeatureFlagConfig::new().with_feature("permissions", FeatureFlag::value("read-only")),
        ));

        let error = flags
            .evaluate_bool("permissions", &FeatureFlagContext::new(), false)
            .unwrap_err();

        assert_eq!(error.kind(), FeatureFlagErrorKind::Transform);
        assert_eq!(error.feature(), Some("permissions"));
    }

    #[derive(Clone, Debug)]
    struct FailingStore;

    impl FeatureFlagStore for FailingStore {
        fn get_configuration(&self) -> FeatureFlagResult<FeatureFlagConfig> {
            Err(FeatureFlagError::store("store unavailable"))
        }
    }

    #[test]
    fn store_errors_fall_back_to_default_values() {
        let flags = FeatureFlags::new(FailingStore);

        assert!(
            flags
                .evaluate_bool("premium_features", &FeatureFlagContext::new(), true)
                .unwrap()
        );
        assert!(
            flags
                .get_enabled_features(&FeatureFlagContext::new())
                .is_empty()
        );
        assert_eq!(
            flags.get_configuration().unwrap_err().kind(),
            FeatureFlagErrorKind::Store
        );
    }
}
