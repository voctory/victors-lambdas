# Feature Flags

The feature flags utility evaluates static and dynamic feature definitions against a request context. It is useful when
configuration alone is not enough and a feature should depend on tenant, tier, region, experiment bucket, or similar
runtime attributes.

This first Rust implementation includes:

- typed feature flag configuration parsing from JSON
- boolean and JSON-valued features
- an in-memory store, an optional AppConfig store, plus `FeatureFlagStore` and `AsyncFeatureFlagStore` traits for custom
  stores
- single-feature evaluation with caller-provided defaults
- enabled-feature listing for boolean flags
- opt-in configuration cache policies for sync and async evaluators
- common context comparators, including equality, ordering, string prefix/suffix, collection membership, and modulo
  ranges
- time-window rules for time-of-day, date-time ranges, and day-of-week matching

Use the Parameters utility for simple static values that do not need rule evaluation.

## AppConfig

Enable `feature-flags-appconfig` on the umbrella crate to load a feature flag configuration from AWS AppConfig. The store
wraps the Parameters utility's AppConfig provider and is evaluated with `AsyncFeatureFlags`.

Use `with_envelope("features")` for a top-level object key or `with_envelope("/runtime/features")` for a JSON Pointer
inside a larger AppConfig document.

## Example

The workspace keeps a buildable snippet in `examples/snippets/feature-flags`:

```rust
use victors_lambdas::feature_flags::{
    FeatureCondition, FeatureFlag, FeatureFlagCachePolicy, FeatureFlagConfig,
    FeatureFlagContext, FeatureFlags, FeatureRule, InMemoryFeatureFlagStore, RuleAction,
};
use serde_json::json;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = FeatureFlagConfig::new().with_feature(
        "premium_features",
        FeatureFlag::boolean(false).with_rule(
            "customer tier equals premium",
            FeatureRule::new(
                true,
                [FeatureCondition::new(
                    RuleAction::Equals,
                    "tier",
                    "premium",
                )],
            ),
        ),
    );
    let feature_flags = FeatureFlags::with_cache_policy(
        InMemoryFeatureFlagStore::from_config(config),
        FeatureFlagCachePolicy::ttl(Duration::from_secs(60)),
    );

    let mut context = FeatureFlagContext::new();
    context.insert("tier".to_owned(), json!("premium"));

    let enabled = feature_flags.evaluate_bool("premium_features", &context, false)?;
    assert!(enabled);

    Ok(())
}
```

JSON configuration uses the same high-level schema shape as the Python utility:

```json
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
  }
}
```
