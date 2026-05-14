//! Compiled `JMESPath` expressions.

use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;

use crate::{
    error::{JmespathError, JmespathResult},
    functions::powertools_runtime,
};

/// Reusable `JMESPath` expression with Powertools decode functions registered.
pub struct JmespathExpression {
    expression: String,
    ast: jmespath::ast::Ast,
    runtime: jmespath::Runtime,
}

impl JmespathExpression {
    /// Compiles a `JMESPath` expression.
    ///
    /// The expression can use standard `JMESPath` functions plus Powertools
    /// helpers: `powertools_json`, `powertools_base64`, and
    /// `powertools_base64_gzip`.
    ///
    /// # Errors
    ///
    /// Returns an error when the expression is not valid `JMESPath`.
    pub fn compile(expression: &str) -> JmespathResult<Self> {
        let runtime = powertools_runtime();
        let compiled = runtime
            .compile(expression)
            .map_err(|error| JmespathError::compile(expression, error))?;

        Ok(Self {
            expression: compiled.as_str().to_owned(),
            ast: compiled.as_ast().clone(),
            runtime,
        })
    }

    /// Returns the source expression.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.expression
    }

    /// Searches serializable data and returns the selected JSON value.
    ///
    /// # Errors
    ///
    /// Returns an error when the data cannot be searched or the selected value
    /// cannot be represented as `serde_json::Value`.
    pub fn search<D>(&self, data: D) -> JmespathResult<Value>
    where
        D: Serialize,
    {
        let expression =
            jmespath::Expression::new(self.expression.clone(), self.ast.clone(), &self.runtime);
        let result = expression
            .search(data)
            .map_err(|error| JmespathError::search(self.as_str(), error))?;

        serde_json::to_value(result.as_ref()).map_err(JmespathError::encode)
    }

    /// Searches serializable data and decodes the selected value into `T`.
    ///
    /// # Errors
    ///
    /// Returns an error when the data cannot be searched, the selected value
    /// cannot be represented as JSON, or the JSON value cannot be decoded into
    /// `T`.
    pub fn search_as<T, D>(&self, data: D) -> JmespathResult<T>
    where
        T: DeserializeOwned,
        D: Serialize,
    {
        let value = self.search(data)?;
        serde_json::from_value(value).map_err(JmespathError::decode)
    }
}

impl std::fmt::Debug for JmespathExpression {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("JmespathExpression")
            .field("expression", &self.expression)
            .finish_non_exhaustive()
    }
}

/// Searches serializable data with a `JMESPath` expression.
///
/// Powertools decode helpers are available to the expression.
///
/// # Errors
///
/// Returns an error when the expression cannot be compiled or evaluated.
pub fn search<D>(expression: &str, data: D) -> JmespathResult<Value>
where
    D: Serialize,
{
    JmespathExpression::compile(expression)?.search(data)
}

/// Searches serializable data with a `JMESPath` expression and decodes the result.
///
/// Powertools decode helpers are available to the expression.
///
/// # Errors
///
/// Returns an error when the expression cannot be compiled or evaluated, or
/// when the selected value cannot be decoded into `T`.
pub fn search_as<T, D>(expression: &str, data: D) -> JmespathResult<T>
where
    T: DeserializeOwned,
    D: Serialize,
{
    JmespathExpression::compile(expression)?.search_as(data)
}

/// Queries data using a Powertools envelope expression.
///
/// This is equivalent to [`search`] but keeps the Python utility's `data`,
/// then `envelope` naming for envelope extraction.
///
/// # Errors
///
/// Returns an error when the envelope expression cannot be compiled or
/// evaluated.
pub fn query<D>(data: D, envelope: &str) -> JmespathResult<Value>
where
    D: Serialize,
{
    search(envelope, data)
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use base64::{Engine as _, engine::general_purpose::STANDARD};
    use flate2::{Compression, write::GzEncoder};
    use serde::Deserialize;
    use serde_json::json;

    use crate::{JmespathErrorKind, search, search_as};

    #[derive(Debug, Deserialize, Eq, PartialEq)]
    struct Order {
        order_id: String,
        quantity: u32,
    }

    #[test]
    fn searches_json_values() {
        let result = search(
            "detail.order_id",
            json!({
                "detail": {
                    "order_id": "order-1",
                    "quantity": 2
                }
            }),
        )
        .expect("expression should search");

        assert_eq!(result, json!("order-1"));
    }

    #[test]
    fn decodes_json_payloads_with_powertools_function() {
        let order = search_as::<Order, _>(
            "powertools_json(body)",
            json!({
                "body": "{\"order_id\":\"order-1\",\"quantity\":2}"
            }),
        )
        .expect("JSON payload should decode");

        assert_eq!(order.order_id, "order-1");
        assert_eq!(order.quantity, 2);
    }

    #[test]
    fn decodes_base64_payloads_with_powertools_function() {
        let body = STANDARD.encode(r#"{"order_id":"order-1","quantity":2}"#);

        let order = search_as::<Order, _>(
            "powertools_json(powertools_base64(body))",
            json!({ "body": body }),
        )
        .expect("base64 JSON payload should decode");

        assert_eq!(order.order_id, "order-1");
        assert_eq!(order.quantity, 2);
    }

    #[test]
    fn decodes_base64_gzip_payloads_with_powertools_function() {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder
            .write_all(br#"{"logEvents":[{"message":"created"}]}"#)
            .expect("gzip input should write");
        let compressed = encoder.finish().expect("gzip payload should finish");

        let result = search(
            "powertools_json(powertools_base64_gzip(data)).logEvents[0].message",
            json!({ "data": STANDARD.encode(compressed) }),
        )
        .expect("base64 gzip JSON payload should decode");

        assert_eq!(result, json!("created"));
    }

    #[test]
    fn reports_compile_errors() {
        let error =
            search("[", json!({})).expect_err("invalid expression should return a compile error");

        assert_eq!(error.kind(), JmespathErrorKind::Compile);
    }
}
