//! AWS `AppConfig` feature flag store.

use serde_json::Value;
use victors_lambdas_parameters::AppConfigProvider;

use crate::{
    AsyncFeatureFlagStore, FeatureFlagConfig, FeatureFlagError, FeatureFlagFuture,
    FeatureFlagResult,
};

/// Asynchronous feature flag store backed by AWS `AppConfig` Data.
#[derive(Clone, Debug)]
pub struct AppConfigFeatureFlagStore {
    provider: AppConfigProvider,
    profile: String,
    envelope: Option<String>,
}

impl AppConfigFeatureFlagStore {
    /// Creates an `AppConfig` feature flag store.
    ///
    /// The store reuses the Parameters utility's `AppConfigProvider` so AWS SDK
    /// configuration and token handling stay in one place.
    #[must_use]
    pub fn new(provider: AppConfigProvider, profile: impl Into<String>) -> Self {
        Self {
            provider,
            profile: profile.into(),
            envelope: None,
        }
    }

    /// Extracts feature flags from a nested configuration envelope.
    ///
    /// Envelopes without a leading slash are treated as top-level object keys,
    /// such as `features`. Envelopes starting with `/` are treated as JSON
    /// Pointers, such as `/runtime/features`.
    #[must_use]
    pub fn with_envelope(mut self, envelope: impl Into<String>) -> Self {
        self.envelope = Some(envelope.into());
        self
    }

    /// Returns the underlying `AppConfig` parameter provider.
    #[must_use]
    pub const fn provider(&self) -> &AppConfigProvider {
        &self.provider
    }

    /// Returns the configuration profile identifier.
    #[must_use]
    pub fn profile(&self) -> &str {
        &self.profile
    }

    /// Returns the configured envelope when one is set.
    #[must_use]
    pub fn envelope(&self) -> Option<&str> {
        self.envelope.as_deref()
    }

    async fn fetch_configuration(&self) -> FeatureFlagResult<FeatureFlagConfig> {
        let Some(configuration) = self
            .provider
            .get_configuration(&self.profile)
            .await
            .map_err(|error| {
                FeatureFlagError::store(format!(
                    "AppConfig profile {} failed: {error}",
                    self.profile
                ))
            })?
        else {
            return Err(FeatureFlagError::store(format!(
                "AppConfig profile {} returned no configuration",
                self.profile
            )));
        };

        configuration_from_bytes(&configuration, self.envelope.as_deref())
    }
}

impl AsyncFeatureFlagStore for AppConfigFeatureFlagStore {
    fn get_configuration(&self) -> FeatureFlagFuture<'_> {
        Box::pin(async move { self.fetch_configuration().await })
    }
}

fn configuration_from_bytes(
    configuration: &[u8],
    envelope: Option<&str>,
) -> FeatureFlagResult<FeatureFlagConfig> {
    let value = serde_json::from_slice(configuration).map_err(|error| {
        FeatureFlagError::configuration(format!("invalid AppConfig feature flag JSON: {error}"))
    })?;
    let value = extract_envelope(value, envelope)?;

    FeatureFlagConfig::from_json_value(value)
}

fn extract_envelope(value: Value, envelope: Option<&str>) -> FeatureFlagResult<Value> {
    let Some(envelope) = envelope
        .map(str::trim)
        .filter(|envelope| !envelope.is_empty())
    else {
        return Ok(value);
    };

    if envelope.starts_with('/') {
        return value.pointer(envelope).cloned().ok_or_else(|| {
            FeatureFlagError::configuration(format!(
                "AppConfig feature flag envelope {envelope} was not found"
            ))
        });
    }

    match value {
        Value::Object(mut object) => object.remove(envelope).ok_or_else(|| {
            FeatureFlagError::configuration(format!(
                "AppConfig feature flag envelope {envelope} was not found"
            ))
        }),
        _ => Err(FeatureFlagError::configuration(format!(
            "AppConfig feature flag envelope {envelope} requires a JSON object"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::configuration_from_bytes;
    use crate::FeatureFlagErrorKind;

    #[test]
    fn parses_root_feature_flag_configuration() {
        let config = configuration_from_bytes(
            br#"{
                "premium_features": {
                    "default": false,
                    "rules": {
                        "customer tier equals premium": {
                            "when_match": true,
                            "conditions": [
                                {"action": "EQUALS", "key": "tier", "value": "premium"}
                            ]
                        }
                    }
                }
            }"#,
            None,
        )
        .unwrap();

        assert!(config.contains("premium_features"));
    }

    #[test]
    fn parses_top_level_envelope() {
        let config = configuration_from_bytes(
            br#"{
                "features": {
                    "ten_percent_off_campaign": {"default": true}
                }
            }"#,
            Some("features"),
        )
        .unwrap();

        assert!(config.contains("ten_percent_off_campaign"));
    }

    #[test]
    fn parses_json_pointer_envelope() {
        let config = configuration_from_bytes(
            br#"{
                "runtime": {
                    "features": {
                        "ten_percent_off_campaign": {"default": true}
                    }
                }
            }"#,
            Some("/runtime/features"),
        )
        .unwrap();

        assert!(config.contains("ten_percent_off_campaign"));
    }

    #[test]
    fn rejects_missing_envelope() {
        let error = configuration_from_bytes(
            br#"{"features": {"ten_percent_off_campaign": {"default": true}}}"#,
            Some("missing"),
        )
        .unwrap_err();

        assert_eq!(error.kind(), FeatureFlagErrorKind::Configuration);
    }

    #[test]
    fn rejects_non_object_envelope() {
        let error = configuration_from_bytes(
            json!(["not", "an", "object"]).to_string().as_bytes(),
            Some("features"),
        )
        .unwrap_err();

        assert_eq!(error.kind(), FeatureFlagErrorKind::Configuration);
    }
}
