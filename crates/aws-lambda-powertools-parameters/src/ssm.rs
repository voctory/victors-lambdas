//! AWS Systems Manager Parameter Store provider.

use aws_sdk_ssm::{
    Client,
    operation::{
        get_parameter::GetParameterOutput, get_parameters::GetParametersOutput,
        get_parameters_by_path::GetParametersByPathOutput, put_parameter::PutParameterOutput,
    },
    types::{Parameter as SdkParameter, ParameterType as SdkParameterType},
};

use crate::{
    AsyncParameterProvider, Parameter, ParameterFuture, ParameterProviderError,
    ParameterProviderResult,
};

const GET_PARAMETERS_MAX_NAMES: usize = 10;

/// Parameters returned from an SSM `GetParameters` request.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SsmParametersByName {
    parameters: Vec<Parameter>,
    invalid_names: Vec<String>,
}

impl SsmParametersByName {
    /// Creates a by-name result from fetched parameters and invalid names.
    #[must_use]
    pub fn new(parameters: Vec<Parameter>, invalid_names: Vec<String>) -> Self {
        Self {
            parameters,
            invalid_names,
        }
    }

    /// Returns successfully fetched parameters.
    #[must_use]
    pub fn parameters(&self) -> &[Parameter] {
        &self.parameters
    }

    /// Returns names SSM reported as invalid or missing.
    #[must_use]
    pub fn invalid_names(&self) -> &[String] {
        &self.invalid_names
    }

    /// Consumes the result and returns fetched parameters.
    #[must_use]
    pub fn into_parameters(self) -> Vec<Parameter> {
        self.parameters
    }

    /// Consumes the result and returns fetched parameters plus invalid names.
    #[must_use]
    pub fn into_parts(self) -> (Vec<Parameter>, Vec<String>) {
        (self.parameters, self.invalid_names)
    }

    fn extend(&mut self, parameters: Vec<Parameter>, invalid_names: &[String]) {
        self.parameters.extend(parameters);
        self.invalid_names.extend(invalid_names.iter().cloned());
    }
}

/// SSM parameter value type used when writing a parameter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SsmParameterType {
    /// Plain string parameter.
    String,
    /// Comma-separated string list parameter.
    StringList,
    /// Encrypted secure string parameter.
    SecureString,
}

impl SsmParameterType {
    fn into_sdk(self) -> SdkParameterType {
        match self {
            Self::String => SdkParameterType::String,
            Self::StringList => SdkParameterType::StringList,
            Self::SecureString => SdkParameterType::SecureString,
        }
    }
}

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

    /// Fetches parameters by full name using SSM `GetParameters`.
    ///
    /// Requests are chunked to SSM's ten-name API limit. Names returned in the
    /// result are the full SSM parameter names.
    ///
    /// # Errors
    ///
    /// Returns [`ParameterProviderError`] when an SSM request fails.
    pub async fn get_parameters_by_name<I, S>(
        &self,
        names: I,
    ) -> ParameterProviderResult<SsmParametersByName>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let names = names
            .into_iter()
            .map(|name| name.as_ref().to_owned())
            .collect::<Vec<_>>();
        let mut result = SsmParametersByName::default();

        for chunk in names.chunks(GET_PARAMETERS_MAX_NAMES) {
            let output = self
                .client
                .get_parameters()
                .set_names(Some(chunk.to_vec()))
                .with_decryption(self.decrypt)
                .send()
                .await
                .map_err(|error| ParameterProviderError::new(chunk.join(","), error.to_string()))?;

            result.extend(parameters_by_name(&output), output.invalid_parameters());
        }

        Ok(result)
    }

    /// Fetches direct child parameters under a path using SSM `GetParametersByPath`.
    ///
    /// Returned names are relative to `path`, matching Powertools' path-based
    /// parameter ergonomics in other runtimes.
    ///
    /// # Errors
    ///
    /// Returns [`ParameterProviderError`] when an SSM request fails.
    pub async fn get_parameters_by_path(
        &self,
        path: &str,
    ) -> ParameterProviderResult<Vec<Parameter>> {
        self.fetch_parameters_by_path(path, false).await
    }

    /// Recursively fetches parameters under a path using SSM `GetParametersByPath`.
    ///
    /// Returned names are relative to `path`, matching Powertools' path-based
    /// parameter ergonomics in other runtimes.
    ///
    /// # Errors
    ///
    /// Returns [`ParameterProviderError`] when an SSM request fails.
    pub async fn get_parameters_by_path_recursive(
        &self,
        path: &str,
    ) -> ParameterProviderResult<Vec<Parameter>> {
        self.fetch_parameters_by_path(path, true).await
    }

    /// Writes a plain string parameter without overwriting an existing value.
    ///
    /// Returns the SSM parameter version assigned by `PutParameter`.
    ///
    /// # Errors
    ///
    /// Returns [`ParameterProviderError`] when the SSM request fails.
    pub async fn set_parameter(&self, name: &str, value: &str) -> ParameterProviderResult<i64> {
        self.set_parameter_with_options(name, value, SsmParameterType::String, false)
            .await
    }

    /// Writes an SSM parameter and returns the assigned version number.
    ///
    /// Use `overwrite` to allow replacing an existing parameter value.
    ///
    /// # Errors
    ///
    /// Returns [`ParameterProviderError`] when the SSM request fails.
    pub async fn set_parameter_with_options(
        &self,
        name: &str,
        value: &str,
        parameter_type: SsmParameterType,
        overwrite: bool,
    ) -> ParameterProviderResult<i64> {
        let output = self
            .client
            .put_parameter()
            .name(name)
            .value(value)
            .r#type(parameter_type.into_sdk())
            .overwrite(overwrite)
            .send()
            .await
            .map_err(|error| ParameterProviderError::new(name, error.to_string()))?;

        Ok(put_parameter_version(&output))
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

    async fn fetch_parameters_by_path(
        &self,
        path: &str,
        recursive: bool,
    ) -> ParameterProviderResult<Vec<Parameter>> {
        let mut next_token = None;
        let mut parameters = Vec::new();

        loop {
            let output = self
                .client
                .get_parameters_by_path()
                .path(path)
                .recursive(recursive)
                .with_decryption(self.decrypt)
                .set_next_token(next_token.take())
                .send()
                .await
                .map_err(|error| ParameterProviderError::new(path, error.to_string()))?;

            parameters.extend(parameters_by_path(&output, path));

            next_token = output.next_token().map(str::to_owned);
            if next_token.is_none() {
                break;
            }
        }

        Ok(parameters)
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

fn parameters_by_name(output: &GetParametersOutput) -> Vec<Parameter> {
    parameters_from_sdk(output.parameters(), None)
}

fn parameters_by_path(output: &GetParametersByPathOutput, path: &str) -> Vec<Parameter> {
    parameters_from_sdk(output.parameters(), Some(path))
}

fn parameters_from_sdk(parameters: &[SdkParameter], path: Option<&str>) -> Vec<Parameter> {
    parameters
        .iter()
        .filter_map(|parameter| parameter_from_sdk(parameter, path))
        .collect()
}

fn parameter_from_sdk(parameter: &SdkParameter, path: Option<&str>) -> Option<Parameter> {
    let name = parameter.name()?;
    let name = path.map_or_else(|| name.to_owned(), |path| relative_name(path, name));
    Some(Parameter::new(name, parameter.value()?))
}

fn relative_name(path: &str, name: &str) -> String {
    name.strip_prefix(path)
        .unwrap_or(name)
        .trim_start_matches('/')
        .to_owned()
}

fn put_parameter_version(output: &PutParameterOutput) -> i64 {
    output.version()
}

#[cfg(test)]
mod tests {
    use aws_sdk_ssm::{
        Client, Config,
        config::{BehaviorVersion, Credentials, Region},
        operation::{
            get_parameter::GetParameterOutput, get_parameters::GetParametersOutput,
            get_parameters_by_path::GetParametersByPathOutput, put_parameter::PutParameterOutput,
        },
        types::{Parameter as SdkParameter, ParameterType as SdkParameterType},
    };

    use crate::Parameter;

    use super::{
        SsmParameterProvider, SsmParameterType, SsmParametersByName, parameter_value,
        parameters_by_name, parameters_by_path, put_parameter_version, relative_name,
    };

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
            .parameter(SdkParameter::builder().name("name").value("value").build())
            .build();

        assert_eq!(parameter_value(&output).as_deref(), Some("value"));
    }

    #[test]
    fn output_parameters_by_name_extract_values_and_invalid_names() {
        let output = GetParametersOutput::builder()
            .parameters(
                SdkParameter::builder()
                    .name("/app/first")
                    .value("1")
                    .build(),
            )
            .parameters(
                SdkParameter::builder()
                    .name("/app/second")
                    .value("2")
                    .build(),
            )
            .invalid_parameters("/app/missing")
            .build();

        let result = SsmParametersByName::new(
            parameters_by_name(&output),
            output.invalid_parameters().to_vec(),
        );

        assert_eq!(
            result.parameters(),
            &[
                Parameter::new("/app/first", "1"),
                Parameter::new("/app/second", "2")
            ]
        );
        assert_eq!(result.invalid_names(), &["/app/missing".to_owned()]);
    }

    #[test]
    fn output_parameters_by_path_use_relative_names() {
        let output = GetParametersByPathOutput::builder()
            .parameters(
                SdkParameter::builder()
                    .name("/service/database/host")
                    .value("localhost")
                    .build(),
            )
            .parameters(
                SdkParameter::builder()
                    .name("/service/database/port")
                    .value("5432")
                    .build(),
            )
            .build();

        assert_eq!(
            parameters_by_path(&output, "/service/database"),
            vec![
                Parameter::new("host", "localhost"),
                Parameter::new("port", "5432")
            ]
        );
    }

    #[test]
    fn parameter_type_maps_to_sdk_type() {
        assert_eq!(
            SsmParameterType::String.into_sdk(),
            SdkParameterType::String
        );
        assert_eq!(
            SsmParameterType::StringList.into_sdk(),
            SdkParameterType::StringList
        );
        assert_eq!(
            SsmParameterType::SecureString.into_sdk(),
            SdkParameterType::SecureString
        );
    }

    #[test]
    fn put_parameter_output_returns_version() {
        let output = PutParameterOutput::builder().version(42).build();

        assert_eq!(put_parameter_version(&output), 42);
    }

    #[test]
    fn relative_names_ignore_non_matching_prefixes() {
        assert_eq!(
            relative_name("/service/database", "/other/name"),
            "other/name"
        );
        assert_eq!(
            relative_name("/service/database", "/service/database/url"),
            "url"
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
