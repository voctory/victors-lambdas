//! AWS Secrets Manager provider.

use aws_sdk_secretsmanager::{Client, operation::get_secret_value::GetSecretValueOutput};
use base64::{Engine as _, engine::general_purpose::STANDARD};

use crate::{
    AsyncParameterProvider, ParameterFuture, ParameterProviderError, ParameterProviderResult,
};

/// Asynchronous provider backed by AWS Secrets Manager.
#[derive(Clone, Debug)]
pub struct SecretsManagerProvider {
    client: Client,
}

impl SecretsManagerProvider {
    /// Creates a Secrets Manager provider from an AWS SDK client.
    ///
    /// The provider accepts a client instead of constructing one internally so
    /// Lambda handlers can choose how they load AWS SDK configuration and so
    /// this crate does not force an `aws-config` dependency on all users.
    #[must_use]
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    /// Returns the underlying AWS SDK client.
    #[must_use]
    pub const fn client(&self) -> &Client {
        &self.client
    }

    async fn fetch_secret(&self, name: &str) -> ParameterProviderResult<Option<String>> {
        let output = self
            .client
            .get_secret_value()
            .secret_id(name)
            .send()
            .await
            .map_err(|error| ParameterProviderError::new(name, error.to_string()))?;

        Ok(secret_value(&output))
    }
}

impl AsyncParameterProvider for SecretsManagerProvider {
    fn get<'a>(&'a self, name: &'a str) -> ParameterFuture<'a> {
        Box::pin(async move { self.fetch_secret(name).await })
    }
}

fn secret_value(output: &GetSecretValueOutput) -> Option<String> {
    if let Some(secret) = output.secret_string() {
        return Some(secret.to_owned());
    }

    output
        .secret_binary()
        .map(|secret| STANDARD.encode(secret.as_ref()))
}

#[cfg(test)]
mod tests {
    use aws_sdk_secretsmanager::{
        Client, Config,
        config::{BehaviorVersion, Credentials, Region},
        operation::get_secret_value::GetSecretValueOutput,
        primitives::Blob,
    };

    use super::{SecretsManagerProvider, secret_value};

    #[test]
    fn provider_keeps_configured_client() {
        let provider = SecretsManagerProvider::new(client());

        assert!(provider.client().config().region().is_some());
    }

    #[test]
    fn output_value_maps_missing_secret_to_none() {
        let output = GetSecretValueOutput::builder().build();

        assert_eq!(secret_value(&output), None);
    }

    #[test]
    fn output_value_prefers_secret_string() {
        let output = GetSecretValueOutput::builder()
            .secret_string("secret")
            .secret_binary(Blob::new(b"ignored".to_vec()))
            .build();

        assert_eq!(secret_value(&output).as_deref(), Some("secret"));
    }

    #[test]
    fn output_value_encodes_secret_binary() {
        let output = GetSecretValueOutput::builder()
            .secret_binary(Blob::new(b"hello".to_vec()))
            .build();

        assert_eq!(secret_value(&output).as_deref(), Some("aGVsbG8="));
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
