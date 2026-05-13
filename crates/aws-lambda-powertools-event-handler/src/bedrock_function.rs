//! Bedrock Agent function-details resolver.

use std::{collections::BTreeMap, fmt};

pub use aws_lambda_powertools_parser::{
    BedrockAgentFunctionAgent, BedrockAgentFunctionEvent, BedrockAgentFunctionParameter,
};
use serde_json::{Value, json};

/// Handler result for Bedrock Agent function tools.
pub type BedrockAgentFunctionHandlerResult<T> = Result<T, BedrockAgentFunctionHandlerError>;

/// Converted Bedrock Agent function parameters keyed by parameter name.
pub type BedrockAgentFunctionParameters = BTreeMap<String, BedrockAgentFunctionParameterValue>;

/// Handler function for a Bedrock Agent function tool.
pub type BedrockAgentFunctionHandler = dyn Fn(
        &BedrockAgentFunctionParameters,
        &BedrockAgentFunctionEvent,
    ) -> BedrockAgentFunctionHandlerResult<BedrockFunctionResult>
    + Send
    + Sync
    + 'static;

/// Converted Bedrock Agent function parameter value.
#[derive(Clone, Debug, PartialEq)]
pub enum BedrockAgentFunctionParameterValue {
    /// String parameter value.
    String(String),
    /// Number parameter value.
    Number(f64),
    /// Integer parameter value.
    Integer(i64),
    /// Boolean parameter value.
    Boolean(bool),
}

impl BedrockAgentFunctionParameterValue {
    /// Returns this value as a string when it was not converted to a scalar.
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(value) => Some(value),
            Self::Number(_) | Self::Integer(_) | Self::Boolean(_) => None,
        }
    }

    /// Returns this value as a number when possible.
    #[must_use]
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Number(value) => Some(*value),
            #[expect(clippy::cast_precision_loss, reason = "parameter convenience accessor")]
            Self::Integer(value) => Some(*value as f64),
            Self::String(_) | Self::Boolean(_) => None,
        }
    }

    /// Returns this value as an integer when possible.
    #[must_use]
    pub const fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Integer(value) => Some(*value),
            Self::String(_) | Self::Number(_) | Self::Boolean(_) => None,
        }
    }

    /// Returns this value as a boolean when possible.
    #[must_use]
    pub const fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Boolean(value) => Some(*value),
            Self::String(_) | Self::Number(_) | Self::Integer(_) => None,
        }
    }

    fn from_parameter(parameter: &BedrockAgentFunctionParameter) -> Self {
        match parameter.parameter_type.as_str() {
            "boolean" => Self::Boolean(parameter.value.eq_ignore_ascii_case("true")),
            "integer" => parameter
                .value
                .parse()
                .map_or_else(|_| Self::String(parameter.value.clone()), Self::Integer),
            "number" => parameter
                .value
                .parse::<f64>()
                .ok()
                .filter(|value| value.is_finite())
                .map_or_else(|| Self::String(parameter.value.clone()), Self::Number),
            _ => Self::String(parameter.value.clone()),
        }
    }
}

/// Response state for a Bedrock Agent function response.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BedrockAgentFunctionResponseState {
    /// Bedrock should fail the current session with a dependency failure.
    Failure,
    /// Bedrock should reprompt the model with the response body.
    Reprompt,
}

impl BedrockAgentFunctionResponseState {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Failure => "FAILURE",
            Self::Reprompt => "REPROMPT",
        }
    }
}

/// Result returned from a Bedrock Agent function tool.
#[derive(Clone, Debug, PartialEq)]
pub enum BedrockFunctionResult {
    /// Raw text body response.
    Text(String),
    /// JSON value serialized into the response body.
    Json(Value),
    /// Fully customized Bedrock function response.
    Response(BedrockFunctionResponse),
}

impl From<&str> for BedrockFunctionResult {
    fn from(body: &str) -> Self {
        Self::Text(body.to_owned())
    }
}

impl From<String> for BedrockFunctionResult {
    fn from(body: String) -> Self {
        Self::Text(body)
    }
}

impl From<Value> for BedrockFunctionResult {
    fn from(value: Value) -> Self {
        Self::Json(value)
    }
}

impl From<BedrockFunctionResponse> for BedrockFunctionResult {
    fn from(response: BedrockFunctionResponse) -> Self {
        Self::Response(response)
    }
}

/// Response builder for Bedrock Agent function-details invocations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BedrockFunctionResponse {
    body: String,
    response_state: Option<BedrockAgentFunctionResponseState>,
    session_attributes: Option<BTreeMap<String, Value>>,
    prompt_session_attributes: Option<BTreeMap<String, Value>>,
    knowledge_bases_configuration: Option<Value>,
}

impl BedrockFunctionResponse {
    /// Creates a text response body.
    #[must_use]
    pub fn new(body: impl Into<String>) -> Self {
        Self {
            body: body.into(),
            response_state: None,
            session_attributes: None,
            prompt_session_attributes: None,
            knowledge_bases_configuration: None,
        }
    }

    /// Creates a text response body.
    #[must_use]
    pub fn text(body: impl Into<String>) -> Self {
        Self::new(body)
    }

    /// Creates a response body from a JSON value.
    #[must_use]
    pub fn json(value: impl Into<Value>) -> Self {
        let value = value.into();
        Self::new(value.to_string())
    }

    /// Creates a failure response body.
    #[must_use]
    pub fn failure(body: impl Into<String>) -> Self {
        Self::new(body).with_response_state(BedrockAgentFunctionResponseState::Failure)
    }

    /// Creates a reprompt response body.
    #[must_use]
    pub fn reprompt(body: impl Into<String>) -> Self {
        Self::new(body).with_response_state(BedrockAgentFunctionResponseState::Reprompt)
    }

    /// Returns the response body.
    #[must_use]
    pub fn body(&self) -> &str {
        &self.body
    }

    /// Returns the response state.
    #[must_use]
    pub const fn response_state(&self) -> Option<BedrockAgentFunctionResponseState> {
        self.response_state
    }

    /// Returns session attributes overriding event attributes.
    #[must_use]
    pub fn session_attributes(&self) -> Option<&BTreeMap<String, Value>> {
        self.session_attributes.as_ref()
    }

    /// Returns prompt session attributes overriding event attributes.
    #[must_use]
    pub fn prompt_session_attributes(&self) -> Option<&BTreeMap<String, Value>> {
        self.prompt_session_attributes.as_ref()
    }

    /// Returns knowledge base configuration overriding event configuration.
    #[must_use]
    pub const fn knowledge_bases_configuration(&self) -> Option<&Value> {
        self.knowledge_bases_configuration.as_ref()
    }

    /// Sets a Bedrock response state.
    #[must_use]
    pub const fn with_response_state(mut self, state: BedrockAgentFunctionResponseState) -> Self {
        self.response_state = Some(state);
        self
    }

    /// Sets response session attributes.
    #[must_use]
    pub fn with_session_attributes(mut self, attributes: BTreeMap<String, Value>) -> Self {
        self.session_attributes = Some(attributes);
        self
    }

    /// Sets response prompt session attributes.
    #[must_use]
    pub fn with_prompt_session_attributes(mut self, attributes: BTreeMap<String, Value>) -> Self {
        self.prompt_session_attributes = Some(attributes);
        self
    }

    /// Sets response knowledge base configuration.
    #[must_use]
    pub fn with_knowledge_bases_configuration(mut self, configuration: Value) -> Self {
        self.knowledge_bases_configuration = Some(configuration);
        self
    }

    /// Builds the Bedrock Agent function response envelope.
    #[must_use]
    pub fn build(&self, event: &BedrockAgentFunctionEvent) -> Value {
        let mut function_response = json!({
            "responseBody": {
                "TEXT": {
                    "body": self.body,
                },
            },
        });

        if let Some(response_state) = self.response_state {
            function_response["responseState"] = json!(response_state.as_str());
        }

        let mut output = json!({
            "messageVersion": "1.0",
            "response": {
                "actionGroup": &event.action_group,
                "function": &event.function_name,
                "functionResponse": function_response,
            },
            "sessionAttributes": self
                .session_attributes
                .as_ref()
                .unwrap_or(&event.session_attributes),
            "promptSessionAttributes": self
                .prompt_session_attributes
                .as_ref()
                .unwrap_or(&event.prompt_session_attributes),
        });

        if let Some(configuration) = self
            .knowledge_bases_configuration
            .as_ref()
            .or(event.knowledge_bases_configuration.as_ref())
        {
            output["knowledgeBasesConfiguration"] = configuration.clone();
        }

        output
    }
}

/// Error returned by Bedrock Agent function tool handlers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BedrockAgentFunctionHandlerError {
    message: String,
}

impl BedrockAgentFunctionHandlerError {
    /// Creates a handler error.
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// Returns the error message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for BedrockAgentFunctionHandlerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for BedrockAgentFunctionHandlerError {}

impl From<&str> for BedrockAgentFunctionHandlerError {
    fn from(message: &str) -> Self {
        Self::new(message)
    }
}

impl From<String> for BedrockAgentFunctionHandlerError {
    fn from(message: String) -> Self {
        Self::new(message)
    }
}

/// Registered Bedrock Agent function route.
pub struct BedrockFunctionRoute {
    name: String,
    description: Option<String>,
    handler: Box<BedrockAgentFunctionHandler>,
}

impl BedrockFunctionRoute {
    /// Creates a Bedrock Agent function route.
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        handler: impl Fn(
            &BedrockAgentFunctionParameters,
            &BedrockAgentFunctionEvent,
        ) -> BedrockAgentFunctionHandlerResult<BedrockFunctionResult>
        + Send
        + Sync
        + 'static,
    ) -> Self {
        Self {
            name: name.into(),
            description: None,
            handler: Box::new(handler),
        }
    }

    /// Returns the Bedrock Agent function name matched by this route.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the optional function description.
    #[must_use]
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// Sets a human-readable function description.
    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    fn handle(
        &self,
        parameters: &BedrockAgentFunctionParameters,
        event: &BedrockAgentFunctionEvent,
    ) -> BedrockAgentFunctionHandlerResult<BedrockFunctionResult> {
        (self.handler)(parameters, event)
    }
}

impl fmt::Debug for BedrockFunctionRoute {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BedrockFunctionRoute")
            .field("name", &self.name)
            .field("description", &self.description)
            .finish_non_exhaustive()
    }
}

/// Routes Bedrock Agent function-details invocations by function name.
#[derive(Default, Debug)]
pub struct BedrockAgentFunctionResolver {
    routes: Vec<BedrockFunctionRoute>,
}

impl BedrockAgentFunctionResolver {
    /// Creates an empty Bedrock Agent function resolver.
    #[must_use]
    pub const fn new() -> Self {
        Self { routes: Vec::new() }
    }

    /// Registers a Bedrock Agent function tool.
    pub fn tool(
        &mut self,
        name: impl Into<String>,
        handler: impl Fn(
            &BedrockAgentFunctionParameters,
            &BedrockAgentFunctionEvent,
        ) -> BedrockAgentFunctionHandlerResult<BedrockFunctionResult>
        + Send
        + Sync
        + 'static,
    ) -> &mut Self {
        self.routes.push(BedrockFunctionRoute::new(name, handler));
        self
    }

    /// Registers a Bedrock Agent function tool with a description.
    pub fn tool_with_description(
        &mut self,
        name: impl Into<String>,
        description: impl Into<String>,
        handler: impl Fn(
            &BedrockAgentFunctionParameters,
            &BedrockAgentFunctionEvent,
        ) -> BedrockAgentFunctionHandlerResult<BedrockFunctionResult>
        + Send
        + Sync
        + 'static,
    ) -> &mut Self {
        self.routes
            .push(BedrockFunctionRoute::new(name, handler).with_description(description));
        self
    }

    /// Returns registered tools in insertion order.
    #[must_use]
    pub fn routes(&self) -> &[BedrockFunctionRoute] {
        &self.routes
    }

    /// Dispatches a Bedrock Agent function-details event.
    ///
    /// When no tool is registered for the event's `function`, the response body
    /// describes the missing tool. Handler errors are converted into a Bedrock
    /// response body so the agent receives the expected response envelope.
    #[must_use]
    pub fn handle(&self, event: &BedrockAgentFunctionEvent) -> Value {
        let Some(route) = self.route_for(&event.function_name) else {
            return BedrockFunctionResponse::text(format!(
                "Error: tool \"{}\" has not been registered.",
                event.function_name
            ))
            .build(event);
        };

        let parameters = parameters_from_event(event);

        match route.handle(&parameters, event) {
            Ok(result) => result.build(event),
            Err(error) => BedrockFunctionResponse::text(format!(
                "Unable to complete tool execution due to {error}"
            ))
            .build(event),
        }
    }

    fn route_for(&self, name: &str) -> Option<&BedrockFunctionRoute> {
        self.routes.iter().rev().find(|route| route.name == name)
    }
}

impl BedrockFunctionResult {
    fn build(self, event: &BedrockAgentFunctionEvent) -> Value {
        match self {
            Self::Text(body) => BedrockFunctionResponse::text(body).build(event),
            Self::Json(value) => BedrockFunctionResponse::json(value).build(event),
            Self::Response(response) => response.build(event),
        }
    }
}

fn parameters_from_event(event: &BedrockAgentFunctionEvent) -> BedrockAgentFunctionParameters {
    event
        .parameters
        .iter()
        .map(|parameter| {
            (
                parameter.name.clone(),
                BedrockAgentFunctionParameterValue::from_parameter(parameter),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde_json::{Value, json};

    use super::{
        BedrockAgentFunctionEvent, BedrockAgentFunctionHandlerError,
        BedrockAgentFunctionParameterValue, BedrockAgentFunctionResolver,
        BedrockAgentFunctionResponseState, BedrockFunctionResponse,
    };

    #[test]
    fn routes_registered_function_tool() {
        let event = event(&json!({
            "function": "get_claim",
            "parameters": [
                { "name": "claim_id", "type": "string", "value": "claim-1" }
            ],
        }));
        let mut resolver = BedrockAgentFunctionResolver::new();
        resolver.tool("get_claim", |parameters, _| {
            let claim_id = parameters
                .get("claim_id")
                .and_then(BedrockAgentFunctionParameterValue::as_str)
                .unwrap_or_default();

            Ok(json!({ "claimId": claim_id, "status": "OPEN" }).into())
        });

        let response = resolver.handle(&event);

        assert_eq!(
            response,
            json!({
                "messageVersion": "1.0",
                "response": {
                    "actionGroup": "Claims",
                    "function": "get_claim",
                    "functionResponse": {
                        "responseBody": {
                            "TEXT": {
                                "body": "{\"claimId\":\"claim-1\",\"status\":\"OPEN\"}"
                            }
                        }
                    }
                },
                "sessionAttributes": {
                    "session": "value"
                },
                "promptSessionAttributes": {}
            })
        );
    }

    #[test]
    fn converts_bedrock_parameter_types() {
        let event = event(&json!({
            "parameters": [
                { "name": "enabled", "type": "boolean", "value": "true" },
                { "name": "count", "type": "integer", "value": "3" },
                { "name": "threshold", "type": "number", "value": "2.5" },
                { "name": "tags", "type": "array", "value": "[\"a\"]" },
                { "name": "invalid", "type": "number", "value": "nan" }
            ],
        }));
        let mut resolver = BedrockAgentFunctionResolver::new();
        resolver.tool("lookup", |parameters, _| {
            assert_eq!(parameters["enabled"].as_bool(), Some(true));
            assert_eq!(parameters["count"].as_i64(), Some(3));
            assert_eq!(parameters["threshold"].as_f64(), Some(2.5));
            assert_eq!(parameters["tags"].as_str(), Some("[\"a\"]"));
            assert_eq!(parameters["invalid"].as_str(), Some("nan"));
            Ok("ok".into())
        });

        let response = resolver.handle(&event);

        assert_eq!(
            response["response"]["functionResponse"]["responseBody"]["TEXT"]["body"],
            "ok"
        );
    }

    #[test]
    fn custom_response_sets_state_and_overrides_context() {
        let event = event(&json!({
            "function": "ask_again",
            "knowledgeBasesConfiguration": [
                { "knowledgeBaseId": "kb-1" }
            ]
        }));
        let mut resolver = BedrockAgentFunctionResolver::new();
        resolver.tool("ask_again", |_, _| {
            Ok(BedrockFunctionResponse::reprompt("need more input")
                .with_session_attributes(BTreeMap::from([("next".to_owned(), json!("question"))]))
                .with_prompt_session_attributes(BTreeMap::from([(
                    "prompt".to_owned(),
                    json!("again"),
                )]))
                .with_knowledge_bases_configuration(json!([
                    { "knowledgeBaseId": "kb-2" }
                ]))
                .into())
        });

        let response = resolver.handle(&event);

        assert_eq!(
            response["response"]["functionResponse"]["responseState"],
            BedrockAgentFunctionResponseState::Reprompt.as_str()
        );
        assert_eq!(response["sessionAttributes"]["next"], "question");
        assert_eq!(response["promptSessionAttributes"]["prompt"], "again");
        assert_eq!(
            response["knowledgeBasesConfiguration"],
            json!([{ "knowledgeBaseId": "kb-2" }])
        );
    }

    #[test]
    fn missing_tool_returns_bedrock_error_response() {
        let event = event(&json!({ "function": "unknown" }));

        let response = BedrockAgentFunctionResolver::new().handle(&event);

        assert_eq!(
            response["response"]["functionResponse"]["responseBody"]["TEXT"]["body"],
            "Error: tool \"unknown\" has not been registered."
        );
    }

    #[test]
    fn handler_error_returns_bedrock_error_response() {
        let event = event(&json!({ "function": "fail" }));
        let mut resolver = BedrockAgentFunctionResolver::new();
        resolver.tool("fail", |_, _| {
            Err(BedrockAgentFunctionHandlerError::new("backend unavailable"))
        });

        let response = resolver.handle(&event);

        assert_eq!(
            response["response"]["functionResponse"]["responseBody"]["TEXT"]["body"],
            "Unable to complete tool execution due to backend unavailable"
        );
    }

    #[test]
    fn later_duplicate_tool_registration_wins() {
        let event = event(&json!({ "function": "lookup" }));
        let mut resolver = BedrockAgentFunctionResolver::new();
        resolver.tool("lookup", |_, _| Ok("first".into()));
        resolver.tool("lookup", |_, _| Ok("second".into()));

        let response = resolver.handle(&event);

        assert_eq!(
            response["response"]["functionResponse"]["responseBody"]["TEXT"]["body"],
            "second"
        );
    }

    #[test]
    fn preserves_event_knowledge_base_configuration() {
        let event = event(&json!({
            "knowledgeBasesConfiguration": [
                { "knowledgeBaseId": "kb-1" }
            ]
        }));
        let mut resolver = BedrockAgentFunctionResolver::new();
        resolver.tool("lookup", |_, _| Ok("ok".into()));

        let response = resolver.handle(&event);

        assert_eq!(
            response["knowledgeBasesConfiguration"],
            json!([{ "knowledgeBaseId": "kb-1" }])
        );
    }

    #[test]
    fn registers_tool_descriptions() {
        let mut resolver = BedrockAgentFunctionResolver::new();
        resolver.tool_with_description("lookup", "Looks up a claim", |_, _| Ok("ok".into()));

        assert_eq!(resolver.routes()[0].name(), "lookup");
        assert_eq!(resolver.routes()[0].description(), Some("Looks up a claim"));
    }

    fn event(patch: &Value) -> BedrockAgentFunctionEvent {
        let mut event = json!({
            "messageVersion": "1.0",
            "agent": {
                "name": "ClaimsAgent",
                "id": "agent-id",
                "alias": "prod",
                "version": "1"
            },
            "inputText": "show claim",
            "sessionId": "session-id",
            "actionGroup": "Claims",
            "function": "lookup",
            "sessionAttributes": {
                "session": "value"
            },
            "promptSessionAttributes": {}
        });

        merge_json(&mut event, patch);
        serde_json::from_value(event).expect("Bedrock Agent function event should deserialize")
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
