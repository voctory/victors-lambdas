//! AWS `AppConfig` Data provider.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex, PoisonError},
    time::{Duration, SystemTime},
};

use aws_sdk_appconfigdata::{
    Client,
    operation::{
        get_latest_configuration::GetLatestConfigurationOutput,
        start_configuration_session::StartConfigurationSessionOutput,
    },
};

use crate::{
    AsyncParameterProvider, ParameterFuture, ParameterProviderError, ParameterProviderResult,
};

const APPCONFIG_TOKEN_TTL: Duration = Duration::from_secs(24 * 60 * 60);

/// Asynchronous provider backed by AWS `AppConfig` Data.
#[derive(Clone, Debug)]
pub struct AppConfigProvider {
    client: Client,
    application: String,
    environment: String,
    sessions: Arc<Mutex<HashMap<String, AppConfigSession>>>,
    last_values: Arc<Mutex<HashMap<String, Vec<u8>>>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct AppConfigSession {
    token: String,
    expires_at: SystemTime,
}

impl AppConfigProvider {
    /// Creates an `AppConfig` provider from an AWS SDK client and `AppConfig` identifiers.
    ///
    /// The provider accepts a client instead of constructing one internally so
    /// Lambda handlers can choose how they load AWS SDK configuration and so
    /// this crate does not force an `aws-config` dependency on all users.
    #[must_use]
    pub fn new(
        client: Client,
        application: impl Into<String>,
        environment: impl Into<String>,
    ) -> Self {
        Self {
            client,
            application: application.into(),
            environment: environment.into(),
            sessions: Arc::new(Mutex::new(HashMap::new())),
            last_values: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Returns the underlying AWS SDK client.
    #[must_use]
    pub const fn client(&self) -> &Client {
        &self.client
    }

    /// Returns the `AppConfig` application identifier.
    #[must_use]
    pub fn application(&self) -> &str {
        &self.application
    }

    /// Returns the `AppConfig` environment identifier.
    #[must_use]
    pub fn environment(&self) -> &str {
        &self.environment
    }

    /// Retrieves the latest configuration bytes for a configuration profile.
    ///
    /// `AppConfig` can return an empty payload when the configuration has not
    /// changed. In that case this provider returns the most recent non-empty
    /// value it has seen for the same profile.
    ///
    /// # Errors
    ///
    /// Returns [`ParameterProviderError`] when the `AppConfig` request fails or
    /// `AppConfig` does not return a usable session token.
    pub async fn get_configuration(
        &self,
        profile: &str,
    ) -> ParameterProviderResult<Option<Vec<u8>>> {
        self.fetch_configuration(profile).await
    }

    async fn fetch_configuration(&self, profile: &str) -> ParameterProviderResult<Option<Vec<u8>>> {
        let now = SystemTime::now();
        let token = match self.session_token(profile, now) {
            Some(token) => token,
            None => self.start_session(profile, now).await?,
        };

        let output = self
            .client
            .get_latest_configuration()
            .configuration_token(token)
            .send()
            .await
            .map_err(|error| ParameterProviderError::new(profile, error.to_string()))?;

        self.store_next_token(profile, output.next_poll_configuration_token(), now);

        Ok(configuration_value(profile, &output, &self.last_values))
    }

    async fn fetch_configuration_string(
        &self,
        profile: &str,
    ) -> ParameterProviderResult<Option<String>> {
        self.fetch_configuration(profile)
            .await?
            .map(|value| {
                String::from_utf8(value)
                    .map_err(|error| ParameterProviderError::new(profile, error.to_string()))
            })
            .transpose()
    }

    async fn start_session(
        &self,
        profile: &str,
        now: SystemTime,
    ) -> ParameterProviderResult<String> {
        let output = self
            .client
            .start_configuration_session()
            .application_identifier(&self.application)
            .environment_identifier(&self.environment)
            .configuration_profile_identifier(profile)
            .send()
            .await
            .map_err(|error| ParameterProviderError::new(profile, error.to_string()))?;

        let token = initial_token(&output).ok_or_else(|| {
            ParameterProviderError::new(profile, "missing initial AppConfig configuration token")
        })?;

        self.store_session(profile, &token, now);
        Ok(token)
    }

    fn session_token(&self, profile: &str, now: SystemTime) -> Option<String> {
        self.sessions
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(profile)
            .filter(|session| session.expires_at > now)
            .map(|session| session.token.clone())
    }

    fn store_session(&self, profile: &str, token: &str, now: SystemTime) {
        self.sessions
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(
                profile.to_owned(),
                AppConfigSession {
                    token: token.to_owned(),
                    expires_at: now + APPCONFIG_TOKEN_TTL,
                },
            );
    }

    fn store_next_token(&self, profile: &str, token: Option<&str>, now: SystemTime) {
        if let Some(token) = token {
            self.store_session(profile, token, now);
        } else {
            self.sessions
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .remove(profile);
        }
    }
}

impl AsyncParameterProvider for AppConfigProvider {
    fn get<'a>(&'a self, name: &'a str) -> ParameterFuture<'a> {
        Box::pin(async move { self.fetch_configuration_string(name).await })
    }
}

fn initial_token(output: &StartConfigurationSessionOutput) -> Option<String> {
    output.initial_configuration_token().map(str::to_owned)
}

fn configuration_value(
    profile: &str,
    output: &GetLatestConfigurationOutput,
    last_values: &Mutex<HashMap<String, Vec<u8>>>,
) -> Option<Vec<u8>> {
    if let Some(configuration) = output
        .configuration()
        .map(std::convert::AsRef::as_ref)
        .filter(|configuration| !configuration.is_empty())
    {
        let value = configuration.to_vec();
        last_values
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(profile.to_owned(), value.clone());

        return Some(value);
    }

    last_values
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get(profile)
        .cloned()
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        sync::Mutex,
        time::{Duration, UNIX_EPOCH},
    };

    use aws_sdk_appconfigdata::{
        Client, Config,
        config::{BehaviorVersion, Credentials, Region},
        operation::{
            get_latest_configuration::GetLatestConfigurationOutput,
            start_configuration_session::StartConfigurationSessionOutput,
        },
        primitives::Blob,
    };

    use super::{AppConfigProvider, configuration_value, initial_token};

    #[test]
    fn provider_keeps_configured_client_and_identifiers() {
        let provider = AppConfigProvider::new(client(), "my-app", "prod");

        assert!(provider.client().config().region().is_some());
        assert_eq!(provider.application(), "my-app");
        assert_eq!(provider.environment(), "prod");
    }

    #[test]
    fn start_session_output_extracts_initial_token() {
        let output = StartConfigurationSessionOutput::builder()
            .initial_configuration_token("token")
            .build();

        assert_eq!(initial_token(&output).as_deref(), Some("token"));
    }

    #[test]
    fn start_session_output_maps_missing_token_to_none() {
        let output = StartConfigurationSessionOutput::builder().build();

        assert_eq!(initial_token(&output), None);
    }

    #[test]
    fn latest_configuration_extracts_bytes_and_updates_cache() {
        let cache = Mutex::new(HashMap::new());
        let output = GetLatestConfigurationOutput::builder()
            .configuration(Blob::new(b"{\"enabled\":true}".to_vec()))
            .next_poll_configuration_token("next-token")
            .build();

        assert_eq!(
            configuration_value("flags", &output, &cache).as_deref(),
            Some(b"{\"enabled\":true}".as_slice())
        );
        assert_eq!(
            cache
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .get("flags")
                .map(Vec::as_slice),
            Some(b"{\"enabled\":true}".as_slice())
        );
    }

    #[test]
    fn latest_configuration_reuses_cached_value_for_empty_payloads() {
        let cache = Mutex::new(HashMap::from([(
            "flags".to_owned(),
            b"{\"enabled\":true}".to_vec(),
        )]));
        let output = GetLatestConfigurationOutput::builder()
            .configuration(Blob::new(Vec::new()))
            .build();

        assert_eq!(
            configuration_value("flags", &output, &cache).as_deref(),
            Some(b"{\"enabled\":true}".as_slice())
        );
    }

    #[test]
    fn expired_session_tokens_are_ignored() {
        let provider = AppConfigProvider::new(client(), "my-app", "prod");
        let now = UNIX_EPOCH + Duration::from_secs(100);

        provider.store_session("flags", "token", now);

        assert_eq!(
            provider.session_token("flags", now),
            Some("token".to_owned())
        );
        assert_eq!(
            provider.session_token("flags", now + Duration::from_secs(24 * 60 * 60 + 1)),
            None
        );
    }

    fn client() -> Client {
        let config = Config::builder()
            .behavior_version(BehaviorVersion::latest())
            .region(Region::new("us-east-1"))
            .credentials_provider(Credentials::new(
                "access-key",
                "secret-key",
                None,
                None,
                "test",
            ))
            .build();

        Client::from_conf(config)
    }
}
