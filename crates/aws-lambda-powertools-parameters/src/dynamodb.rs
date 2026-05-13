//! Amazon `DynamoDB` parameter provider.

use std::collections::HashMap;

use aws_sdk_dynamodb::{
    Client,
    operation::{get_item::GetItemOutput, query::QueryOutput},
    types::AttributeValue,
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde_json::{Map, Value};

use crate::{
    AsyncParameterProvider, Parameter, ParameterFuture, ParameterProviderError,
    ParameterProviderResult,
};

/// Asynchronous provider backed by an Amazon `DynamoDB` table.
#[derive(Clone, Debug)]
pub struct DynamoDbParameterProvider {
    client: Client,
    table_name: String,
    key_attr: String,
    sort_attr: String,
    value_attr: String,
}

impl DynamoDbParameterProvider {
    /// Creates a `DynamoDB` parameter provider with default attribute names.
    ///
    /// The default partition key is `id`, the default sort key is `sk`, and
    /// the default value attribute is `value`.
    ///
    /// The provider accepts a client instead of constructing one internally so
    /// Lambda handlers can choose how they load AWS SDK configuration and so
    /// this crate does not force an `aws-config` dependency on all users.
    #[must_use]
    pub fn new(client: Client, table_name: impl Into<String>) -> Self {
        Self {
            client,
            table_name: table_name.into(),
            key_attr: "id".to_owned(),
            sort_attr: "sk".to_owned(),
            value_attr: "value".to_owned(),
        }
    }

    /// Returns a copy of the provider with a custom partition key attribute.
    #[must_use]
    pub fn with_key_attr(mut self, key_attr: impl Into<String>) -> Self {
        self.key_attr = key_attr.into();
        self
    }

    /// Returns a copy of the provider with a custom sort key attribute.
    #[must_use]
    pub fn with_sort_attr(mut self, sort_attr: impl Into<String>) -> Self {
        self.sort_attr = sort_attr.into();
        self
    }

    /// Returns a copy of the provider with a custom value attribute.
    #[must_use]
    pub fn with_value_attr(mut self, value_attr: impl Into<String>) -> Self {
        self.value_attr = value_attr.into();
        self
    }

    /// Returns the underlying AWS SDK client.
    #[must_use]
    pub const fn client(&self) -> &Client {
        &self.client
    }

    /// Returns the `DynamoDB` table name.
    #[must_use]
    pub fn table_name(&self) -> &str {
        &self.table_name
    }

    /// Returns the partition key attribute name.
    #[must_use]
    pub fn key_attr(&self) -> &str {
        &self.key_attr
    }

    /// Returns the sort key attribute name.
    #[must_use]
    pub fn sort_attr(&self) -> &str {
        &self.sort_attr
    }

    /// Returns the value attribute name.
    #[must_use]
    pub fn value_attr(&self) -> &str {
        &self.value_attr
    }

    /// Retrieves all parameters under a partition key path.
    ///
    /// The returned parameter names come from the configured sort key
    /// attribute, matching Powertools' `DynamoDB` provider behavior in other
    /// runtimes.
    ///
    /// # Errors
    ///
    /// Returns [`ParameterProviderError`] when a `DynamoDB` request fails.
    pub async fn get_parameters_by_path(
        &self,
        path: &str,
    ) -> ParameterProviderResult<Vec<Parameter>> {
        let mut exclusive_start_key = None;
        let mut parameters = Vec::new();

        loop {
            let output = self
                .client
                .query()
                .table_name(&self.table_name)
                .key_condition_expression("#key = :key")
                .expression_attribute_names("#key", &self.key_attr)
                .expression_attribute_names("#sort", &self.sort_attr)
                .expression_attribute_names("#value", &self.value_attr)
                .expression_attribute_values(":key", AttributeValue::S(path.to_owned()))
                .projection_expression("#sort, #value")
                .set_exclusive_start_key(exclusive_start_key.take())
                .send()
                .await
                .map_err(|error| ParameterProviderError::new(path, error.to_string()))?;

            parameters.extend(parameters_from_query(
                &output,
                &self.sort_attr,
                &self.value_attr,
            ));

            exclusive_start_key = output.last_evaluated_key().cloned();
            if exclusive_start_key.is_none() {
                break;
            }
        }

        Ok(parameters)
    }

    async fn fetch_parameter(&self, name: &str) -> ParameterProviderResult<Option<String>> {
        let output = self
            .client
            .get_item()
            .table_name(&self.table_name)
            .key(&self.key_attr, AttributeValue::S(name.to_owned()))
            .projection_expression("#value")
            .expression_attribute_names("#value", &self.value_attr)
            .send()
            .await
            .map_err(|error| ParameterProviderError::new(name, error.to_string()))?;

        Ok(parameter_value(&output, &self.value_attr))
    }
}

impl AsyncParameterProvider for DynamoDbParameterProvider {
    fn get<'a>(&'a self, name: &'a str) -> ParameterFuture<'a> {
        Box::pin(async move { self.fetch_parameter(name).await })
    }
}

fn parameter_value(output: &GetItemOutput, value_attr: &str) -> Option<String> {
    output
        .item()
        .and_then(|item| item.get(value_attr))
        .and_then(attribute_to_parameter_value)
}

fn parameters_from_query(
    output: &QueryOutput,
    sort_attr: &str,
    value_attr: &str,
) -> Vec<Parameter> {
    output
        .items()
        .iter()
        .filter_map(|item| parameter_from_item(item, sort_attr, value_attr))
        .collect()
}

fn parameter_from_item(
    item: &HashMap<String, AttributeValue>,
    sort_attr: &str,
    value_attr: &str,
) -> Option<Parameter> {
    let name = item.get(sort_attr).and_then(attribute_to_parameter_value)?;
    let value = item
        .get(value_attr)
        .and_then(attribute_to_parameter_value)?;
    Some(Parameter::new(name, value))
}

fn attribute_to_parameter_value(attribute: &AttributeValue) -> Option<String> {
    match attribute {
        AttributeValue::B(value) => Some(STANDARD.encode(value.as_ref())),
        AttributeValue::Bool(value) => Some(value.to_string()),
        AttributeValue::N(value) | AttributeValue::S(value) => Some(value.clone()),
        AttributeValue::Null(_) => Some("null".to_owned()),
        AttributeValue::Bs(_)
        | AttributeValue::L(_)
        | AttributeValue::M(_)
        | AttributeValue::Ns(_)
        | AttributeValue::Ss(_) => serde_json::to_string(&attribute_to_json_value(attribute)?).ok(),
        _ => None,
    }
}

fn attribute_to_json_value(attribute: &AttributeValue) -> Option<Value> {
    match attribute {
        AttributeValue::B(value) => Some(Value::String(STANDARD.encode(value.as_ref()))),
        AttributeValue::Bool(value) => Some(Value::Bool(*value)),
        AttributeValue::Bs(values) => Some(Value::Array(
            values
                .iter()
                .map(|value| Value::String(STANDARD.encode(value.as_ref())))
                .collect(),
        )),
        AttributeValue::L(values) => values
            .iter()
            .map(attribute_to_json_value)
            .collect::<Option<Vec<_>>>()
            .map(Value::Array),
        AttributeValue::M(values) => values
            .iter()
            .map(|(key, value)| Some((key.clone(), attribute_to_json_value(value)?)))
            .collect::<Option<Map<_, _>>>()
            .map(Value::Object),
        AttributeValue::N(value) => Some(number_to_json_value(value)),
        AttributeValue::Ns(values) => Some(Value::Array(
            values
                .iter()
                .map(|value| number_to_json_value(value))
                .collect(),
        )),
        AttributeValue::Null(_) => Some(Value::Null),
        AttributeValue::S(value) => Some(Value::String(value.clone())),
        AttributeValue::Ss(values) => Some(Value::Array(
            values.iter().cloned().map(Value::String).collect(),
        )),
        _ => None,
    }
}

fn number_to_json_value(value: &str) -> Value {
    serde_json::from_str(value).unwrap_or_else(|_| Value::String(value.to_owned()))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use aws_sdk_dynamodb::{
        Client, Config,
        config::{BehaviorVersion, Credentials, Region},
        operation::{get_item::GetItemOutput, query::QueryOutput},
        primitives::Blob,
        types::AttributeValue,
    };
    use serde_json::{Value, json};

    use crate::Parameter;

    use super::{
        DynamoDbParameterProvider, attribute_to_parameter_value, parameter_value,
        parameters_from_query,
    };

    #[test]
    fn provider_uses_default_and_custom_attributes() {
        let provider = DynamoDbParameterProvider::new(client(), "parameters")
            .with_key_attr("pk")
            .with_sort_attr("name")
            .with_value_attr("body");

        assert!(provider.client().config().region().is_some());
        assert_eq!(provider.table_name(), "parameters");
        assert_eq!(provider.key_attr(), "pk");
        assert_eq!(provider.sort_attr(), "name");
        assert_eq!(provider.value_attr(), "body");
    }

    #[test]
    fn get_item_output_extracts_value_attribute() {
        let output = GetItemOutput::builder()
            .item("value", AttributeValue::S("stored".to_owned()))
            .build();

        assert_eq!(parameter_value(&output, "value").as_deref(), Some("stored"));
    }

    #[test]
    fn get_item_output_maps_missing_items_to_none() {
        let output = GetItemOutput::builder().build();

        assert_eq!(parameter_value(&output, "value"), None);
    }

    #[test]
    fn query_output_extracts_sort_key_names_and_values() {
        let output = QueryOutput::builder()
            .items(HashMap::from([
                ("sk".to_owned(), AttributeValue::S("first".to_owned())),
                ("value".to_owned(), AttributeValue::S("one".to_owned())),
            ]))
            .items(HashMap::from([
                ("sk".to_owned(), AttributeValue::S("second".to_owned())),
                ("value".to_owned(), AttributeValue::N("2".to_owned())),
            ]))
            .build();

        assert_eq!(
            parameters_from_query(&output, "sk", "value"),
            vec![
                Parameter::new("first", "one"),
                Parameter::new("second", "2")
            ]
        );
    }

    #[test]
    fn attribute_values_render_parameter_strings() {
        assert_eq!(
            attribute_to_parameter_value(&AttributeValue::S("plain".to_owned())).as_deref(),
            Some("plain")
        );
        assert_eq!(
            attribute_to_parameter_value(&AttributeValue::N("42".to_owned())).as_deref(),
            Some("42")
        );
        assert_eq!(
            attribute_to_parameter_value(&AttributeValue::Bool(true)).as_deref(),
            Some("true")
        );
        assert_eq!(
            attribute_to_parameter_value(&AttributeValue::B(Blob::new(b"hi".to_vec()))).as_deref(),
            Some("aGk=")
        );
    }

    #[test]
    fn collection_attribute_values_render_json() {
        let value = AttributeValue::M(HashMap::from([
            ("enabled".to_owned(), AttributeValue::Bool(true)),
            ("limit".to_owned(), AttributeValue::N("5".to_owned())),
            (
                "labels".to_owned(),
                AttributeValue::Ss(vec!["stable".to_owned(), "beta".to_owned()]),
            ),
        ]));
        let rendered = attribute_to_parameter_value(&value).expect("value should render");

        assert_eq!(
            serde_json::from_str::<Value>(&rendered).expect("rendered JSON should parse"),
            json!({
                "enabled": true,
                "limit": 5,
                "labels": ["stable", "beta"]
            })
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
