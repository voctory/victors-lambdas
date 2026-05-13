//! AWS Systems Manager Parameter Store provider.

use aws_sdk_ssm::{Client, operation::get_parameter::GetParameterOutput};

use crate::{
    AsyncParameterProvider, ParameterFuture, ParameterProviderError, ParameterProviderResult,
};

/// Asynchronous provider backed by AWS Systems Manager Parameter Store.
#[derive(Clone, Debug)]
pub struct SsmParameterProvider {
    client: Client,
    decrypt: bool,
}

impl SsmParameterProvider {
    /// Creates an SSM parameter provider from an AWS SDK client.
    ///
    /// The provider accepts a client instead of constructing one internally so
    /// Lambda handlers can choose how they load AWS SDK configuration and so
    /// this crate does not force an `aws-config` dependency on all users.
    #[must_use]
    pub fn new(client: Client) -> Self {
        Self {
            client,
            decrypt: false,
        }
    }

    /// Returns a copy of the provider with secure string decryption enabled or disabled.
    #[must_use]
    pub fn with_decryption(mut self, decrypt: bool) -> Self {
        self.decrypt = decrypt;
        self
    }

    /// Returns whether secure string decryption is enabled.
    #[must_use]
    pub const fn decrypt(&self) -> bool {
        self.decrypt
    }

    /// Returns the underlying AWS SDK client.
    #[must_use]
    pub const fn client(&self) -> &Client {
        &self.client
    }

    async fn fetch_parameter(&self, name: &str) -> ParameterProviderResult<Option<String>> {
        let output = self
            .client
            .get_parameter()
            .name(name)
            .with_decryption(self.decrypt)
            .send()
            .await
            .map_err(|error| ParameterProviderError::new(name, error.to_string()))?;

        Ok(parameter_value(&output))
    }
}

impl AsyncParameterProvider for SsmParameterProvider {
    fn get<'a>(&'a self, name: &'a str) -> ParameterFuture<'a> {
        Box::pin(async move { self.fetch_parameter(name).await })
    }
}

fn parameter_value(output: &GetParameterOutput) -> Option<String> {
    output
        .parameter()
        .and_then(|parameter| parameter.value())
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use aws_sdk_ssm::{
        Client, Config,
        config::{BehaviorVersion, Credentials, Region},
        operation::get_parameter::GetParameterOutput,
        types::Parameter,
    };

    use super::{SsmParameterProvider, parameter_value};

    #[test]
    fn provider_configures_decryption() {
        let provider = SsmParameterProvider::new(client());

        assert!(!provider.decrypt());
        let provider = provider.with_decryption(true);
        assert!(provider.decrypt());
        assert!(provider.client().config().region().is_some());
    }

    #[test]
    fn output_value_maps_missing_parameters_to_none() {
        let output = GetParameterOutput::builder().build();

        assert_eq!(parameter_value(&output), None);
    }

    #[test]
    fn output_value_extracts_parameter_value() {
        let output = GetParameterOutput::builder()
            .parameter(Parameter::builder().name("name").value("value").build())
            .build();

        assert_eq!(parameter_value(&output).as_deref(), Some("value"));
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
