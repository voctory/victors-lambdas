//! Bedrock Agent event models and envelopes.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;

use crate::{EventParser, ParseError, ParsedEvent};

/// Agent metadata in a Bedrock Agent function-details invocation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BedrockAgentFunctionAgent {
    /// Agent name.
    pub name: String,
    /// Agent identifier.
    pub id: String,
    /// Agent alias.
    pub alias: String,
    /// Agent version.
    pub version: String,
}

/// Raw parameter supplied in a Bedrock Agent function-details invocation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BedrockAgentFunctionParameter {
    /// Parameter name.
    pub name: String,
    /// Parameter type as defined in the Bedrock Agent function details.
    #[serde(rename = "type")]
    pub parameter_type: String,
    /// Parameter value supplied by Bedrock.
    pub value: String,
}

/// Compatibility alias for the Bedrock Agent parser model name.
pub type BedrockAgentModel = BedrockAgentFunctionAgent;

/// Compatibility alias for a Bedrock Agent property parser model name.
pub type BedrockAgentPropertyModel = BedrockAgentFunctionParameter;

/// Request media payload supplied to a Bedrock Agent `OpenAPI` action group.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BedrockAgentRequestMedia {
    /// Request body properties supplied for this media type.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub properties: Vec<BedrockAgentPropertyModel>,
}

/// Request body supplied to a Bedrock Agent `OpenAPI` action group.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BedrockAgentRequestBody {
    /// Media type keyed request body content.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub content: BTreeMap<String, BedrockAgentRequestMedia>,
}

/// Bedrock Agent `OpenAPI` action group invocation event.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BedrockAgentEvent {
    /// Message format version.
    pub message_version: String,
    /// Agent metadata.
    pub agent: BedrockAgentModel,
    /// User input for the conversation turn.
    pub input_text: String,
    /// Unique Bedrock Agent session identifier.
    pub session_id: String,
    /// Action group name.
    pub action_group: String,
    /// `OpenAPI` path selected by Bedrock.
    pub api_path: String,
    /// HTTP method selected by Bedrock.
    pub http_method: String,
    /// `OpenAPI` parameters supplied by Bedrock.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parameters: Vec<BedrockAgentPropertyModel>,
    /// `OpenAPI` request body supplied by Bedrock.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_body: Option<BedrockAgentRequestBody>,
    /// Session attributes supplied by Bedrock.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub session_attributes: BTreeMap<String, Value>,
    /// Prompt session attributes supplied by Bedrock.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub prompt_session_attributes: BTreeMap<String, Value>,
}

/// Bedrock Agent function-details invocation event.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BedrockAgentFunctionEvent {
    /// Message format version.
    pub message_version: String,
    /// Agent metadata.
    pub agent: BedrockAgentFunctionAgent,
    /// User input for the conversation turn.
    pub input_text: String,
    /// Unique Bedrock Agent session identifier.
    pub session_id: String,
    /// Action group name.
    pub action_group: String,
    /// Function name selected by Bedrock.
    #[serde(rename = "function")]
    pub function_name: String,
    /// Function parameters supplied by Bedrock.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parameters: Vec<BedrockAgentFunctionParameter>,
    /// Session attributes supplied by Bedrock.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub session_attributes: BTreeMap<String, Value>,
    /// Prompt session attributes supplied by Bedrock.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub prompt_session_attributes: BTreeMap<String, Value>,
    /// Optional knowledge base retrieval configuration to preserve in responses.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub knowledge_bases_configuration: Option<Value>,
}

/// Compatibility alias for the Bedrock Agent `OpenAPI` request media parser model name.
pub type BedrockAgentRequestMediaModel = BedrockAgentRequestMedia;

/// Compatibility alias for the Bedrock Agent `OpenAPI` request body parser model name.
pub type BedrockAgentRequestBodyModel = BedrockAgentRequestBody;

/// Compatibility alias for the Bedrock Agent `OpenAPI` event parser model name.
pub type BedrockAgentEventModel = BedrockAgentEvent;

/// Compatibility alias for the Bedrock Agent function-details parser model name.
pub type BedrockAgentFunctionEventModel = BedrockAgentFunctionEvent;

impl EventParser {
    /// Parses the JSON `inputText` payload from a Bedrock Agent `OpenAPI` event.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] when `inputText` is not valid JSON for `T`.
    pub fn parse_bedrock_agent_openapi_input<T>(
        &self,
        event: &BedrockAgentEvent,
    ) -> Result<ParsedEvent<T>, ParseError>
    where
        T: DeserializeOwned,
    {
        self.parse_json_str(event.input_text.as_str())
    }

    /// Parses the JSON `inputText` payload from a Bedrock Agent function-details event.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] when `inputText` is not valid JSON for `T`.
    pub fn parse_bedrock_agent_function_input<T>(
        &self,
        event: &BedrockAgentFunctionEvent,
    ) -> Result<ParsedEvent<T>, ParseError>
    where
        T: DeserializeOwned,
    {
        self.parse_json_str(event.input_text.as_str())
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;
    use serde_json::{Value, json};

    use super::{
        BedrockAgentEvent, BedrockAgentEventModel, BedrockAgentFunctionEvent,
        BedrockAgentFunctionEventModel,
    };
    use crate::EventParser;

    #[derive(Debug, Deserialize, Eq, PartialEq)]
    struct VacationRequest {
        username: String,
        days: u8,
    }

    #[test]
    fn parses_bedrock_agent_openapi_event() {
        let event = serde_json::from_value::<BedrockAgentEventModel>(openapi_event(&json!({})))
            .expect("Bedrock Agent OpenAPI event should parse");

        assert_eq!(event.message_version, "1.0");
        assert_eq!(event.session_id, "session-id");
        assert_eq!(event.input_text, r#"{"username":"Jane","days":3}"#);
        assert_eq!(event.action_group, "TimeOff");
        assert_eq!(event.api_path, "/time-off");
        assert_eq!(event.http_method, "POST");
        assert_eq!(event.session_attributes["employeeId"], "EMP123");
        assert_eq!(event.prompt_session_attributes["requestType"], "vacation");
        assert_eq!(event.agent.id, "agent-id");
        assert_eq!(event.parameters.len(), 1);
        assert_eq!(event.parameters[0].name, "department");
        assert_eq!(event.parameters[0].parameter_type, "string");
        assert_eq!(event.parameters[0].value, "Engineering");

        let request_body = event.request_body.expect("request body should be present");
        let media = request_body
            .content
            .get("application/json")
            .expect("JSON media should be present");

        assert_eq!(media.properties.len(), 1);
        assert_eq!(media.properties[0].name, "reason");
        assert_eq!(media.properties[0].parameter_type, "string");
        assert_eq!(media.properties[0].value, "conference");
    }

    #[test]
    fn parses_bedrock_agent_openapi_input_text() {
        let event = serde_json::from_value::<BedrockAgentEvent>(openapi_event(&json!({})))
            .expect("Bedrock Agent OpenAPI event should parse");

        let parsed = EventParser::new()
            .parse_bedrock_agent_openapi_input::<VacationRequest>(&event)
            .expect("inputText should parse");

        assert_eq!(
            parsed.into_payload(),
            VacationRequest {
                username: "Jane".to_owned(),
                days: 3,
            }
        );
    }

    #[test]
    fn defaults_missing_bedrock_agent_openapi_optionals() {
        let event = serde_json::from_value::<BedrockAgentEvent>(json!({
            "messageVersion": "1.0",
            "agent": {
                "name": "TimeOffAgent",
                "id": "agent-id",
                "alias": "prod",
                "version": "1"
            },
            "inputText": "{\"username\":\"Jane\",\"days\":3}",
            "sessionId": "session-id",
            "actionGroup": "TimeOff",
            "apiPath": "/time-off",
            "httpMethod": "POST"
        }))
        .expect("Bedrock Agent OpenAPI event should parse");

        assert!(event.parameters.is_empty());
        assert!(event.request_body.is_none());
        assert!(event.session_attributes.is_empty());
        assert!(event.prompt_session_attributes.is_empty());
    }

    #[test]
    fn parses_bedrock_agent_function_event() {
        let event = serde_json::from_value::<BedrockAgentFunctionEventModel>(event(&json!({})))
            .expect("Bedrock Agent function event should parse");

        assert_eq!(event.message_version, "1.0");
        assert_eq!(event.session_id, "session-id");
        assert_eq!(event.input_text, r#"{"username":"Jane","days":3}"#);
        assert_eq!(event.action_group, "TimeOff");
        assert_eq!(event.function_name, "request_vacation");
        assert_eq!(event.session_attributes["employeeId"], "EMP123");
        assert_eq!(event.prompt_session_attributes["requestType"], "vacation");
        assert_eq!(event.agent.id, "agent-id");
        assert_eq!(event.parameters.len(), 2);
        assert_eq!(event.parameters[0].name, "start_date");
        assert_eq!(event.parameters[0].parameter_type, "string");
        assert_eq!(event.parameters[0].value, "2026-06-01");
    }

    #[test]
    fn parses_bedrock_agent_function_input_text() {
        let event = serde_json::from_value::<BedrockAgentFunctionEvent>(event(&json!({})))
            .expect("Bedrock Agent function event should parse");

        let parsed = EventParser::new()
            .parse_bedrock_agent_function_input::<VacationRequest>(&event)
            .expect("inputText should parse");

        assert_eq!(
            parsed.into_payload(),
            VacationRequest {
                username: "Jane".to_owned(),
                days: 3,
            }
        );
    }

    #[test]
    fn rejects_invalid_bedrock_agent_function_input_text() {
        let event = serde_json::from_value::<BedrockAgentFunctionEvent>(event(&json!({
            "inputText": "not-json",
        })))
        .expect("Bedrock Agent function event should parse");

        let error = EventParser::new()
            .parse_bedrock_agent_function_input::<VacationRequest>(&event)
            .expect_err("invalid JSON should fail");

        assert!(error.to_string().contains("expected ident"));
    }

    fn openapi_event(patch: &Value) -> Value {
        let mut event = json!({
            "messageVersion": "1.0",
            "agent": {
                "name": "TimeOffAgent",
                "id": "agent-id",
                "alias": "prod",
                "version": "1"
            },
            "inputText": "{\"username\":\"Jane\",\"days\":3}",
            "sessionId": "session-id",
            "actionGroup": "TimeOff",
            "apiPath": "/time-off",
            "httpMethod": "POST",
            "parameters": [
                { "name": "department", "type": "string", "value": "Engineering" }
            ],
            "requestBody": {
                "content": {
                    "application/json": {
                        "properties": [
                            { "name": "reason", "type": "string", "value": "conference" }
                        ]
                    }
                }
            },
            "sessionAttributes": {
                "employeeId": "EMP123"
            },
            "promptSessionAttributes": {
                "requestType": "vacation"
            }
        });

        merge_json(&mut event, patch);
        event
    }

    fn event(patch: &Value) -> Value {
        let mut event = json!({
            "messageVersion": "1.0",
            "agent": {
                "name": "TimeOffAgent",
                "id": "agent-id",
                "alias": "prod",
                "version": "1"
            },
            "inputText": "{\"username\":\"Jane\",\"days\":3}",
            "sessionId": "session-id",
            "actionGroup": "TimeOff",
            "function": "request_vacation",
            "parameters": [
                { "name": "start_date", "type": "string", "value": "2026-06-01" },
                { "name": "end_date", "type": "string", "value": "2026-06-05" }
            ],
            "sessionAttributes": {
                "employeeId": "EMP123"
            },
            "promptSessionAttributes": {
                "requestType": "vacation"
            }
        });

        merge_json(&mut event, patch);
        event
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
