//! Bedrock Agent event adapters.

use std::{fmt, str::FromStr};

use aws_lambda_events::event::bedrock_agent_runtime::AgentEvent;
use serde_json::{Value, json};

use crate::{Method, ParseMethodError, Request, Response, Router};

/// Error returned by Bedrock Agent event adapters.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BedrockAgentAdapterError {
    /// The event HTTP method is not supported by the router.
    InvalidMethod(ParseMethodError),
    /// The event request body could not be serialized into request bytes.
    RequestBodySerialization(String),
}

impl fmt::Display for BedrockAgentAdapterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMethod(error) => error.fmt(formatter),
            Self::RequestBodySerialization(message) => {
                write!(
                    formatter,
                    "failed to serialize Bedrock Agent request body: {message}"
                )
            }
        }
    }
}

impl std::error::Error for BedrockAgentAdapterError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidMethod(error) => Some(error),
            Self::RequestBodySerialization(_) => None,
        }
    }
}

impl From<ParseMethodError> for BedrockAgentAdapterError {
    fn from(error: ParseMethodError) -> Self {
        Self::InvalidMethod(error)
    }
}

/// Result returned by Bedrock Agent adapters.
pub type BedrockAgentAdapterResult<T> = Result<T, BedrockAgentAdapterError>;

impl Router {
    /// Routes a Bedrock Agent event through this HTTP router.
    ///
    /// # Errors
    ///
    /// Returns [`BedrockAgentAdapterError`] when the event cannot be converted
    /// into a router request.
    pub fn handle_bedrock_agent(&self, event: &AgentEvent) -> BedrockAgentAdapterResult<Value> {
        let request = request_from_bedrock_agent(event)?;
        let response = self.handle(request);
        Ok(response_to_bedrock_agent(event, &response))
    }
}

/// Converts a Bedrock Agent event into an HTTP router request.
///
/// Bedrock Agent operation parameters are exposed as both query string and path
/// parameters so handlers can use the existing request accessors.
///
/// # Errors
///
/// Returns [`BedrockAgentAdapterError`] when the event has an unsupported HTTP
/// method or an unserializable request body.
pub fn request_from_bedrock_agent(event: &AgentEvent) -> BedrockAgentAdapterResult<Request> {
    let method = Method::from_str(&event.http_method)?;
    let mut request = Request::new(method, &event.api_path);

    if let Some(parameters) = &event.parameters {
        for parameter in parameters {
            request = request
                .with_query_string_parameter(&parameter.name, &parameter.value)
                .with_path_param(&parameter.name, &parameter.value);
        }
    }

    if let Some(body) = &event.request_body {
        let body = serde_json::to_vec(body).map_err(|error| {
            BedrockAgentAdapterError::RequestBodySerialization(error.to_string())
        })?;
        request = request.with_body(body);
    }

    Ok(request)
}

/// Builds a Bedrock Agent response envelope from a router response.
#[must_use]
pub fn response_to_bedrock_agent(event: &AgentEvent, response: &Response) -> Value {
    let content_type = response
        .header("content-type")
        .unwrap_or("application/json");
    let body = String::from_utf8_lossy(response.body());
    let mut output = json!({
        "messageVersion": "1.0",
        "response": {
            "actionGroup": event.action_group,
            "apiPath": event.api_path,
            "httpMethod": event.http_method,
            "httpStatusCode": response.status_code(),
            "responseBody": {
                content_type: {
                    "body": body,
                },
            },
        },
    });

    if !event.session_attributes.is_empty() {
        output["sessionAttributes"] = json!(event.session_attributes);
    }

    if !event.prompt_session_attributes.is_empty() {
        output["promptSessionAttributes"] = json!(event.prompt_session_attributes);
    }

    output
}

#[cfg(test)]
mod tests {
    use aws_lambda_events::event::bedrock_agent_runtime::AgentEvent;
    use serde_json::{Value, json};

    use super::{BedrockAgentAdapterError, request_from_bedrock_agent, response_to_bedrock_agent};
    use crate::{Method, Response, Router};

    #[test]
    fn converts_bedrock_agent_event_to_router_request() {
        let event = event(
            "GET",
            "/claims",
            &json!({
                "parameters": [
                    { "name": "claim_id", "type": "string", "value": "claim-1" }
                ]
            }),
        );

        let request = request_from_bedrock_agent(&event).expect("request should convert");

        assert_eq!(request.method(), Method::Get);
        assert_eq!(request.path(), "/claims");
        assert_eq!(request.query_string_parameter("claim_id"), Some("claim-1"));
        assert_eq!(request.path_param("claim_id"), Some("claim-1"));
    }

    #[test]
    fn routes_bedrock_agent_event_and_builds_response() {
        let event = event(
            "GET",
            "/claims",
            &json!({
                "parameters": [
                    { "name": "claim_id", "type": "string", "value": "claim-1" }
                ]
            }),
        );
        let mut router = Router::new();
        router.get("/claims", |request| {
            Response::ok(format!(
                "claim {}",
                request.path_param("claim_id").unwrap_or_default()
            ))
            .with_header("content-type", "text/plain")
        });

        let response = router
            .handle_bedrock_agent(&event)
            .expect("route should resolve");

        assert_eq!(
            response,
            json!({
                "messageVersion": "1.0",
                "response": {
                    "actionGroup": "Claims",
                    "apiPath": "/claims",
                    "httpMethod": "GET",
                    "httpStatusCode": 200,
                    "responseBody": {
                        "text/plain": {
                            "body": "claim claim-1",
                        },
                    },
                },
                "sessionAttributes": {
                    "session": "value",
                },
            })
        );
    }

    #[test]
    fn response_envelope_defaults_to_json_content_type() {
        let event = event("POST", "/claims", &json!({}));
        let response = response_to_bedrock_agent(&event, &Response::ok(r#"{"ok":true}"#));

        assert_eq!(
            response["response"]["responseBody"],
            json!({
                "application/json": {
                    "body": r#"{"ok":true}"#,
                },
            })
        );
    }

    #[test]
    fn invalid_bedrock_agent_method_returns_adapter_error() {
        let event = event("TRACE2", "/claims", &json!({}));

        let error = request_from_bedrock_agent(&event).expect_err("method should fail");

        assert!(matches!(error, BedrockAgentAdapterError::InvalidMethod(_)));
    }

    fn event(method: &str, path: &str, extra: &Value) -> AgentEvent {
        let mut event = json!({
            "messageVersion": "1.0",
            "agent": {
                "name": "ClaimsAgent",
                "id": "agent-id",
                "alias": "prod",
                "version": "1",
            },
            "inputText": "show claim",
            "sessionId": "session-id",
            "actionGroup": "Claims",
            "apiPath": path,
            "httpMethod": method,
            "sessionAttributes": {
                "session": "value",
            },
            "promptSessionAttributes": {},
        });

        merge_json(&mut event, extra);
        serde_json::from_value(event).expect("Bedrock Agent event should deserialize")
    }

    fn merge_json(target: &mut Value, patch: &Value) {
        let Some(target) = target.as_object_mut() else {
            return;
        };
        let Some(patch) = patch.as_object() else {
            return;
        };

        for (key, value) in patch {
            target.insert(key.clone(), value.clone());
        }
    }
}
