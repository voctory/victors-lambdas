//! Feature flag configuration model.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{FeatureFlagError, FeatureFlagResult};

fn default_boolean_type() -> bool {
    true
}

/// Feature flag configuration keyed by feature name.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FeatureFlagConfig {
    features: IndexMap<String, FeatureFlag>,
}

impl FeatureFlagConfig {
    /// Creates an empty feature flag configuration.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Parses feature flag configuration from a JSON string.
    ///
    /// # Errors
    ///
    /// Returns a configuration error when the JSON document does not match the
    /// feature flag schema.
    pub fn from_json_str(input: &str) -> FeatureFlagResult<Self> {
        serde_json::from_str(input).map_err(|error| {
            FeatureFlagError::configuration(format!("invalid feature flag configuration: {error}"))
        })
    }

    /// Parses feature flag configuration from a JSON value.
    ///
    /// # Errors
    ///
    /// Returns a configuration error when the JSON value does not match the
    /// feature flag schema.
    pub fn from_json_value(value: Value) -> FeatureFlagResult<Self> {
        serde_json::from_value(value).map_err(|error| {
            FeatureFlagError::configuration(format!("invalid feature flag configuration: {error}"))
        })
    }

    /// Adds or replaces a feature and returns the updated configuration.
    #[must_use]
    pub fn with_feature(mut self, name: impl Into<String>, feature: FeatureFlag) -> Self {
        self.insert(name, feature);
        self
    }

    /// Inserts or replaces a feature.
    pub fn insert(&mut self, name: impl Into<String>, feature: FeatureFlag) -> Option<FeatureFlag> {
        self.features.insert(name.into(), feature)
    }

    /// Returns a feature by name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&FeatureFlag> {
        self.features.get(name)
    }

    /// Returns whether a feature exists.
    #[must_use]
    pub fn contains(&self, name: &str) -> bool {
        self.features.contains_key(name)
    }

    /// Returns the number of configured features.
    #[must_use]
    pub fn len(&self) -> usize {
        self.features.len()
    }

    /// Returns whether the configuration contains no features.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.features.is_empty()
    }

    /// Iterates over configured features in source or insertion order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &FeatureFlag)> {
        self.features
            .iter()
            .map(|(name, feature)| (name.as_str(), feature))
    }
}

impl<K> FromIterator<(K, FeatureFlag)> for FeatureFlagConfig
where
    K: Into<String>,
{
    fn from_iter<T: IntoIterator<Item = (K, FeatureFlag)>>(iter: T) -> Self {
        let mut config = Self::new();
        for (name, feature) in iter {
            config.insert(name, feature);
        }
        config
    }
}

/// A single feature flag definition.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FeatureFlag {
    #[serde(rename = "default")]
    default_value: Value,
    #[serde(default = "default_boolean_type", rename = "boolean_type")]
    boolean_type: bool,
    #[serde(default)]
    rules: IndexMap<String, FeatureRule>,
}

impl FeatureFlag {
    /// Creates a boolean feature flag.
    #[must_use]
    pub fn boolean(default_value: bool) -> Self {
        Self {
            default_value: Value::Bool(default_value),
            boolean_type: true,
            rules: IndexMap::new(),
        }
    }

    /// Creates a feature flag that returns arbitrary JSON values.
    #[must_use]
    pub fn value(default_value: impl Into<Value>) -> Self {
        Self {
            default_value: default_value.into(),
            boolean_type: false,
            rules: IndexMap::new(),
        }
    }

    /// Adds or replaces a rule and returns the updated feature.
    #[must_use]
    pub fn with_rule(mut self, name: impl Into<String>, rule: FeatureRule) -> Self {
        self.insert_rule(name, rule);
        self
    }

    /// Inserts or replaces a rule.
    pub fn insert_rule(
        &mut self,
        name: impl Into<String>,
        rule: FeatureRule,
    ) -> Option<FeatureRule> {
        self.rules.insert(name.into(), rule)
    }

    /// Returns the default value for this feature.
    #[must_use]
    pub fn default_value(&self) -> &Value {
        &self.default_value
    }

    /// Returns whether this feature is intended to produce boolean values.
    #[must_use]
    pub const fn is_boolean(&self) -> bool {
        self.boolean_type
    }

    /// Returns configured rules in source or insertion order.
    pub fn rules(&self) -> impl Iterator<Item = (&str, &FeatureRule)> {
        self.rules.iter().map(|(name, rule)| (name.as_str(), rule))
    }

    /// Returns whether this feature has no configured rules.
    #[must_use]
    pub fn has_rules(&self) -> bool {
        !self.rules.is_empty()
    }
}

/// A named rule's return value and all conditions that must match.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FeatureRule {
    #[serde(rename = "when_match")]
    when_match: Value,
    #[serde(default)]
    conditions: Vec<FeatureCondition>,
}

impl FeatureRule {
    /// Creates a feature rule.
    #[must_use]
    pub fn new<I>(when_match: impl Into<Value>, conditions: I) -> Self
    where
        I: IntoIterator<Item = FeatureCondition>,
    {
        Self {
            when_match: when_match.into(),
            conditions: conditions.into_iter().collect(),
        }
    }

    /// Adds a condition and returns the updated rule.
    #[must_use]
    pub fn with_condition(mut self, condition: FeatureCondition) -> Self {
        self.conditions.push(condition);
        self
    }

    /// Returns the value to use when this rule matches.
    #[must_use]
    pub fn when_match(&self) -> &Value {
        &self.when_match
    }

    /// Returns the conditions that must all match for this rule.
    pub fn conditions(&self) -> impl Iterator<Item = &FeatureCondition> {
        self.conditions.iter()
    }

    /// Returns whether the rule has no conditions.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.conditions.is_empty()
    }
}

/// A single condition within a feature rule.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FeatureCondition {
    action: RuleAction,
    key: String,
    value: Value,
}

impl FeatureCondition {
    /// Creates a feature rule condition.
    #[must_use]
    pub fn new(action: RuleAction, key: impl Into<String>, value: impl Into<Value>) -> Self {
        Self {
            action,
            key: key.into(),
            value: value.into(),
        }
    }

    /// Returns the comparison action.
    #[must_use]
    pub const fn action(&self) -> RuleAction {
        self.action
    }

    /// Returns the context key this condition reads.
    #[must_use]
    pub fn key(&self) -> &str {
        &self.key
    }

    /// Returns the comparison value.
    #[must_use]
    pub fn value(&self) -> &Value {
        &self.value
    }
}

/// Comparison action used by a feature rule condition.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub enum RuleAction {
    /// Match when context value equals condition value.
    #[serde(rename = "EQUALS")]
    Equals,
    /// Match when context value does not equal condition value.
    #[serde(rename = "NOT_EQUALS")]
    NotEquals,
    /// Match when context value is greater than condition value.
    #[serde(rename = "KEY_GREATER_THAN_VALUE")]
    KeyGreaterThanValue,
    /// Match when context value is greater than or equal to condition value.
    #[serde(rename = "KEY_GREATER_THAN_OR_EQUAL_VALUE")]
    KeyGreaterThanOrEqualValue,
    /// Match when context value is less than condition value.
    #[serde(rename = "KEY_LESS_THAN_VALUE")]
    KeyLessThanValue,
    /// Match when context value is less than or equal to condition value.
    #[serde(rename = "KEY_LESS_THAN_OR_EQUAL_VALUE")]
    KeyLessThanOrEqualValue,
    /// Match when context string starts with condition string.
    #[serde(rename = "STARTSWITH")]
    StartsWith,
    /// Match when context string ends with condition string.
    #[serde(rename = "ENDSWITH")]
    EndsWith,
    /// Match when condition value contains context value.
    #[serde(rename = "IN")]
    In,
    /// Match when condition value does not contain context value.
    #[serde(rename = "NOT_IN")]
    NotIn,
    /// Match when condition value contains context value.
    #[serde(rename = "KEY_IN_VALUE")]
    KeyInValue,
    /// Match when condition value does not contain context value.
    #[serde(rename = "KEY_NOT_IN_VALUE")]
    KeyNotInValue,
    /// Match when context value contains condition value.
    #[serde(rename = "VALUE_IN_KEY")]
    ValueInKey,
    /// Match when context value does not contain condition value.
    #[serde(rename = "VALUE_NOT_IN_KEY")]
    ValueNotInKey,
    /// Match when all items in the context array are in the condition array.
    #[serde(rename = "ALL_IN_VALUE")]
    AllInValue,
    /// Match when any item in the context array is in the condition array.
    #[serde(rename = "ANY_IN_VALUE")]
    AnyInValue,
    /// Match when no items in the context array are in the condition array.
    #[serde(rename = "NONE_IN_VALUE")]
    NoneInValue,
    /// Match when `context % BASE` is within inclusive `START` and `END` bounds.
    #[serde(rename = "MODULO_RANGE")]
    ModuloRange,
}
