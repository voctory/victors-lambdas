//! Feature flags snippet for documentation.

use aws_lambda_powertools::feature_flags::{
    FeatureCondition, FeatureFlag, FeatureFlagCachePolicy, FeatureFlagConfig, FeatureFlagContext,
    FeatureFlags, FeatureRule, InMemoryFeatureFlagStore, RuleAction,
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
                [FeatureCondition::new(RuleAction::Equals, "tier", "premium")],
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
