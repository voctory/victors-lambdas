# Feature Flags

The feature flags utility evaluates static and dynamic feature definitions against a request context. It is useful when
configuration alone is not enough and a feature should depend on tenant, tier, region, experiment bucket, or similar
runtime attributes.

This first Rust implementation includes:

- typed feature flag configuration parsing from JSON
- boolean and JSON-valued features
- an in-memory store and a `FeatureFlagStore` trait for custom stores
- single-feature evaluation with caller-provided defaults
- enabled-feature listing for boolean flags
- common context comparators, including equality, ordering, string prefix/suffix, collection membership, and modulo
  ranges

AppConfig-backed stores and time-window rules are still planned. Use the Parameters utility for simple static values
that do not need rule evaluation.

## Example

The workspace keeps a buildable snippet in `examples/snippets/feature-flags`:

```rust
use aws_lambda_powertools::feature_flags::{
    FeatureCondition, FeatureFlag, FeatureFlagConfig, FeatureFlagContext, FeatureFlags,
    FeatureRule, InMemoryFeatureFlagStore, RuleAction,
};
use serde_json::json;

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
    let feature_flags = FeatureFlags::new(InMemoryFeatureFlagStore::from_config(config));

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
