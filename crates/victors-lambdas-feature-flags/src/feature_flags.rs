//! Feature flag evaluator.

use std::{
    cmp::Ordering,
    sync::{Mutex, PoisonError},
    time::SystemTime,
};

use chrono::{Datelike, NaiveDateTime, NaiveTime, Timelike, Utc, Weekday};
use chrono_tz::Tz;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Map, Value};

use crate::{
    FeatureFlag, FeatureFlagCachePolicy, FeatureFlagConfig, FeatureFlagError, FeatureFlagResult,
    FeatureFlagStore, FeatureRule, RuleAction, cache::CachedFeatureFlagConfig,
};

/// Context values used while evaluating dynamic feature flag rules.
pub type FeatureFlagContext = Map<String, Value>;

/// Evaluates feature flags against a configured store.
#[derive(Debug)]
pub struct FeatureFlags<S> {
    store: S,
    cache_policy: FeatureFlagCachePolicy,
    cache: Mutex<Option<CachedFeatureFlagConfig>>,
}

impl<S> FeatureFlags<S> {
    /// Creates a feature flag evaluator with a store provider.
    #[must_use]
    pub fn new(store: S) -> Self {
        Self::with_cache_policy(store, FeatureFlagCachePolicy::default())
    }

    /// Creates a feature flag evaluator with a store provider and cache policy.
    #[must_use]
    pub const fn with_cache_policy(store: S, cache_policy: FeatureFlagCachePolicy) -> Self {
        Self {
            store,
            cache_policy,
            cache: Mutex::new(None),
        }
    }

    /// Returns the configured store provider.
    #[must_use]
    pub const fn store(&self) -> &S {
        &self.store
    }

    /// Returns the feature flag configuration cache policy.
    #[must_use]
    pub const fn cache_policy(&self) -> FeatureFlagCachePolicy {
        self.cache_policy
    }

    /// Clears cached feature flag configuration.
    pub fn clear_cache(&self) {
        self.cache
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
    }

    /// Returns the number of cached feature flag configurations.
    #[must_use]
    pub fn cache_len(&self) -> usize {
        usize::from(
            self.cache
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .is_some(),
        )
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
        self.get_configuration_at(SystemTime::now())
    }

    /// Gets feature flag configuration from the store, bypassing any cached value.
    ///
    /// When the store returns configuration, the cache is updated with that
    /// value if caching is enabled.
    ///
    /// # Errors
    ///
    /// Returns a store or configuration error when the store cannot provide a
    /// feature flag configuration.
    pub fn get_configuration_force(&self) -> FeatureFlagResult<FeatureFlagConfig> {
        self.fetch_configuration_at(SystemTime::now())
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
        let Ok(config) = self.get_configuration() else {
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
        let Ok(config) = self.get_configuration() else {
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

    fn get_configuration_at(&self, now: SystemTime) -> FeatureFlagResult<FeatureFlagConfig> {
        if let Some(config) = self.cached_configuration(now) {
            return Ok(config);
        }

        self.fetch_configuration_at(now)
    }

    fn fetch_configuration_at(&self, now: SystemTime) -> FeatureFlagResult<FeatureFlagConfig> {
        let config = self.store.get_configuration()?;
        self.store_cached_configuration(config.clone(), now);
        Ok(config)
    }

    fn cached_configuration(&self, now: SystemTime) -> Option<FeatureFlagConfig> {
        if !self.cache_policy.enabled() {
            return None;
        }

        self.cache
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .as_ref()
            .filter(|cached| self.cache_policy.is_fresh(cached.cached_at, now))
            .map(|cached| cached.config.clone())
    }

    fn store_cached_configuration(&self, config: FeatureFlagConfig, now: SystemTime) {
        if !self.cache_policy.enabled() {
            return;
        }

        self.cache
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .replace(CachedFeatureFlagConfig::new(config, now));
    }
}

impl<S> Clone for FeatureFlags<S>
where
    S: Clone,
{
    fn clone(&self) -> Self {
        Self {
            store: self.store.clone(),
            cache_policy: self.cache_policy,
            cache: Mutex::new(
                self.cache
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .clone(),
            ),
        }
    }
}

impl<S> PartialEq for FeatureFlags<S>
where
    S: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.store == other.store && self.cache_policy == other.cache_policy
    }
}

impl<S> Eq for FeatureFlags<S> where S: Eq {}

pub(crate) fn evaluate_feature(feature: &FeatureFlag, context: &FeatureFlagContext) -> Value {
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
        && rule.conditions().all(|condition| {
            if !condition.action().reads_context() {
                return matches_condition(condition.action(), &Value::Null, condition.value());
            }

            match context.get(condition.key()) {
                Some(context_value) => {
                    matches_condition(condition.action(), context_value, condition.value())
                }
                None => false,
            }
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
        RuleAction::ScheduleBetweenTimeRange => compare_time_range(condition_value),
        RuleAction::ScheduleBetweenDateTimeRange => compare_datetime_range(condition_value),
        RuleAction::ScheduleBetweenDaysOfWeek => compare_days_of_week(condition_value),
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

fn compare_time_range(condition_value: &Value) -> bool {
    let Some(condition) = condition_value.as_object() else {
        return false;
    };
    let (Some(start), Some(end), Some(now)) = (
        condition_time(condition, "START").map(minutes_since_midnight),
        condition_time(condition, "END").map(minutes_since_midnight),
        now_in_timezone(condition).map(|now| minutes_since_midnight(now.time())),
    ) else {
        return false;
    };

    if end < start {
        start <= now || now <= end
    } else {
        start <= now && now <= end
    }
}

fn compare_datetime_range(condition_value: &Value) -> bool {
    let Some(condition) = condition_value.as_object() else {
        return false;
    };
    let (Some(start), Some(end), Some(now)) = (
        condition_datetime(condition, "START"),
        condition_datetime(condition, "END"),
        now_in_timezone(condition).map(|now| now.naive_local()),
    ) else {
        return false;
    };

    start <= now && now <= end
}

fn compare_days_of_week(condition_value: &Value) -> bool {
    let Some(condition) = condition_value.as_object() else {
        return false;
    };
    let Some(now) = now_in_timezone(condition) else {
        return false;
    };
    let Some(days) = condition.get("DAYS").and_then(Value::as_array) else {
        return false;
    };
    let current_day = weekday_name(now.weekday());

    days.iter()
        .any(|day| day.as_str().is_some_and(|day| day == current_day))
}

fn condition_time(condition: &Map<String, Value>, key: &str) -> Option<NaiveTime> {
    NaiveTime::parse_from_str(condition.get(key)?.as_str()?, "%H:%M").ok()
}

fn minutes_since_midnight(time: NaiveTime) -> u32 {
    (time.hour() * 60) + time.minute()
}

fn condition_datetime(condition: &Map<String, Value>, key: &str) -> Option<NaiveDateTime> {
    NaiveDateTime::parse_from_str(condition.get(key)?.as_str()?, "%Y-%m-%dT%H:%M:%S%.f").ok()
}

fn now_in_timezone(condition: &Map<String, Value>) -> Option<chrono::DateTime<Tz>> {
    let timezone = condition
        .get("TIMEZONE")
        .and_then(Value::as_str)
        .unwrap_or("UTC")
        .parse::<Tz>()
        .ok()?;

    Some(Utc::now().with_timezone(&timezone))
}

fn weekday_name(weekday: Weekday) -> &'static str {
    match weekday {
        Weekday::Mon => "MONDAY",
        Weekday::Tue => "TUESDAY",
        Weekday::Wed => "WEDNESDAY",
        Weekday::Thu => "THURSDAY",
        Weekday::Fri => "FRIDAY",
        Weekday::Sat => "SATURDAY",
        Weekday::Sun => "SUNDAY",
    }
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{Arc, Mutex},
        time::Duration,
    };

    use serde::{Deserialize, Serialize};
    use serde_json::{Value, json};

    use super::{FeatureFlagContext, FeatureFlags};
    use crate::{
        FeatureCondition, FeatureFlag, FeatureFlagCachePolicy, FeatureFlagConfig, FeatureFlagError,
        FeatureFlagErrorKind, FeatureFlagResult, FeatureFlagStore, FeatureRule,
        InMemoryFeatureFlagStore, RuleAction,
    };

    fn context(pairs: impl IntoIterator<Item = (&'static str, Value)>) -> FeatureFlagContext {
        pairs
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect()
    }

    #[derive(Clone, Debug)]
    struct CountingStore {
        config: FeatureFlagConfig,
        calls: Arc<Mutex<usize>>,
    }

    impl CountingStore {
        fn new(config: FeatureFlagConfig) -> Self {
            Self {
                config,
                calls: Arc::new(Mutex::new(0)),
            }
        }

        fn calls(&self) -> usize {
            *self.calls.lock().unwrap()
        }
    }

    impl FeatureFlagStore for CountingStore {
        fn get_configuration(&self) -> FeatureFlagResult<FeatureFlagConfig> {
            *self.calls.lock().unwrap() += 1;
            Ok(self.config.clone())
        }
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
    fn supports_time_window_rules() {
        let every_day = json!({
            "DAYS": [
                "SUNDAY",
                "MONDAY",
                "TUESDAY",
                "WEDNESDAY",
                "THURSDAY",
                "FRIDAY",
                "SATURDAY"
            ],
            "TIMEZONE": "UTC"
        });
        let feature = FeatureFlag::boolean(false)
            .with_rule(
                "all day",
                FeatureRule::new(
                    true,
                    [FeatureCondition::new(
                        RuleAction::ScheduleBetweenTimeRange,
                        "CURRENT_TIME",
                        json!({"START": "00:00", "END": "23:59", "TIMEZONE": "UTC"}),
                    )],
                ),
            )
            .with_rule(
                "current century",
                FeatureRule::new(
                    true,
                    [FeatureCondition::new(
                        RuleAction::ScheduleBetweenDateTimeRange,
                        "CURRENT_DATETIME",
                        json!({
                            "START": "2000-01-01T00:00:00",
                            "END": "2999-12-31T23:59:59",
                            "TIMEZONE": "UTC"
                        }),
                    )],
                ),
            )
            .with_rule(
                "every day",
                FeatureRule::new(
                    true,
                    [FeatureCondition::new(
                        RuleAction::ScheduleBetweenDaysOfWeek,
                        "CURRENT_DAY_OF_WEEK",
                        every_day,
                    )],
                ),
            );
        let flags = FeatureFlags::new(InMemoryFeatureFlagStore::from_config(
            FeatureFlagConfig::new().with_feature("scheduled_feature", feature),
        ));

        assert!(
            flags
                .evaluate_bool("scheduled_feature", &FeatureFlagContext::new(), false)
                .unwrap()
        );
    }

    #[test]
    fn expired_time_window_rule_does_not_match() {
        let feature = FeatureFlag::boolean(false).with_rule(
            "expired launch",
            FeatureRule::new(
                true,
                [FeatureCondition::new(
                    RuleAction::ScheduleBetweenDateTimeRange,
                    "CURRENT_DATETIME",
                    json!({
                        "START": "2000-01-01T00:00:00",
                        "END": "2001-01-01T00:00:00",
                        "TIMEZONE": "UTC"
                    }),
                )],
            ),
        );
        let flags = FeatureFlags::new(InMemoryFeatureFlagStore::from_config(
            FeatureFlagConfig::new().with_feature("scheduled_feature", feature),
        ));

        assert!(
            !flags
                .evaluate_bool("scheduled_feature", &FeatureFlagContext::new(), true)
                .unwrap()
        );
    }

    #[test]
    fn disabled_cache_fetches_configuration_each_time() {
        let store = CountingStore::new(
            FeatureFlagConfig::new().with_feature("always_on", FeatureFlag::boolean(true)),
        );
        let flags = FeatureFlags::new(store.clone());

        assert!(
            flags
                .evaluate_bool("always_on", &FeatureFlagContext::new(), false)
                .unwrap()
        );
        assert!(
            flags
                .evaluate_bool("always_on", &FeatureFlagContext::new(), false)
                .unwrap()
        );

        assert_eq!(store.calls(), 2);
        assert_eq!(flags.cache_policy(), FeatureFlagCachePolicy::disabled());
        assert_eq!(flags.cache_len(), 0);
    }

    #[test]
    fn enabled_cache_reuses_configuration_until_cleared_or_forced() {
        let store = CountingStore::new(
            FeatureFlagConfig::new().with_feature("always_on", FeatureFlag::boolean(true)),
        );
        let flags = FeatureFlags::with_cache_policy(
            store.clone(),
            FeatureFlagCachePolicy::ttl(Duration::from_secs(60)),
        );

        assert!(
            flags
                .evaluate_bool("always_on", &FeatureFlagContext::new(), false)
                .unwrap()
        );
        assert!(
            flags
                .evaluate_bool("always_on", &FeatureFlagContext::new(), false)
                .unwrap()
        );
        assert_eq!(store.calls(), 1);
        assert_eq!(flags.cache_len(), 1);

        flags.get_configuration_force().unwrap();
        assert_eq!(store.calls(), 2);
        assert_eq!(flags.cache_len(), 1);

        flags.clear_cache();
        assert_eq!(flags.cache_len(), 0);
        flags.get_configuration().unwrap();
        assert_eq!(store.calls(), 3);
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
