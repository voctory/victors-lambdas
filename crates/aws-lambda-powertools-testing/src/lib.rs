//! Testing helpers for Powertools Lambda Rust.

mod context;
mod fixture;
mod handler;
#[cfg(feature = "streaming")]
mod streaming;

#[cfg(feature = "feature-flags")]
pub use aws_lambda_powertools_feature_flags::InMemoryFeatureFlagStore as FeatureFlagStoreStub;
#[cfg(feature = "idempotency")]
pub use aws_lambda_powertools_idempotency::InMemoryIdempotencyStore as IdempotencyStoreStub;
pub use aws_lambda_powertools_parameters::InMemoryParameterProvider as ParameterProviderStub;
pub use context::LambdaContextStub;
pub use fixture::{FixtureError, load_json_fixture, read_fixture, read_fixture_bytes};
pub use handler::HandlerHarness;
#[cfg(feature = "streaming")]
pub use streaming::S3ObjectClientStub;

#[cfg(test)]
mod tests {
    #[cfg(feature = "feature-flags")]
    #[test]
    fn feature_flag_store_stub_evaluates_config() {
        use aws_lambda_powertools_feature_flags::{
            FeatureFlag, FeatureFlagConfig, FeatureFlagContext, FeatureFlags,
        };

        use super::FeatureFlagStoreStub;

        let store = FeatureFlagStoreStub::from_config(
            FeatureFlagConfig::new().with_feature("beta", FeatureFlag::boolean(true)),
        );
        let flags = FeatureFlags::new(store);

        assert!(
            flags
                .evaluate_bool("beta", &FeatureFlagContext::new(), false)
                .expect("feature flag evaluates")
        );
    }

    #[cfg(feature = "idempotency")]
    #[test]
    fn idempotency_store_stub_keeps_records() {
        use std::time::{Duration, UNIX_EPOCH};

        use aws_lambda_powertools_idempotency::{IdempotencyRecord, IdempotencyStore};

        use super::IdempotencyStoreStub;

        let record =
            IdempotencyRecord::completed_until("request-1", UNIX_EPOCH + Duration::from_secs(60));
        let mut store = IdempotencyStoreStub::new().with_record(record.clone());

        assert_eq!(
            store.get(record.key()).expect("record loads"),
            Some(record.clone())
        );
        assert_eq!(
            store
                .clear_expired(UNIX_EPOCH + Duration::from_secs(61))
                .expect("expired record clears"),
            1
        );
        assert!(store.is_empty());
    }
}
