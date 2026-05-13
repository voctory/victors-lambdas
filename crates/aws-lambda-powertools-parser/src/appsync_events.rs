//! AWS `AppSync` Events models.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Operation that invoked an AWS `AppSync` Events handler.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AppSyncEventsOperation {
    /// Publish operation.
    Publish,
    /// Subscribe operation.
    Subscribe,
}

/// Request metadata for AWS `AppSync` Events handlers.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSyncEventsRequest {
    /// Headers `AppSync` exposed to the handler.
    ///
    /// Header values are JSON values so repeated headers can be represented as
    /// arrays when `AppSync` supplies them that way.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub headers: BTreeMap<String, Value>,
    /// Custom domain name used for the request, when one exists.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domain_name: Option<String>,
}

/// Channel information for an AWS `AppSync` Events handler invocation.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSyncEventsChannel {
    /// Full channel path, such as `/default/room-1`.
    pub path: String,
    /// Channel path segments without the leading slash.
    pub segments: Vec<String>,
}

/// Channel namespace information for an AWS `AppSync` Events handler invocation.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSyncEventsChannelNamespace {
    /// Channel namespace name.
    pub name: String,
}

/// Operation metadata for AWS `AppSync` Events handlers.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSyncEventsInfo {
    /// Channel that received the operation.
    pub channel: AppSyncEventsChannel,
    /// Namespace containing the channel.
    pub channel_namespace: AppSyncEventsChannelNamespace,
    /// Operation type.
    pub operation: AppSyncEventsOperation,
}

/// AWS Lambda authorizer identity for AWS `AppSync` Events handlers.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSyncEventsLambdaIdentity {
    /// Handler context returned by the Lambda authorizer.
    pub handler_context: Value,
}

/// IAM identity for AWS `AppSync` Events handlers.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSyncEventsIamIdentity {
    /// AWS account ID of the caller.
    pub account_id: String,
    /// Amazon Cognito identity pool ID, when credentials came from an identity pool.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cognito_identity_pool_id: Option<String>,
    /// Amazon Cognito identity ID, when credentials came from an identity pool.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cognito_identity_id: Option<String>,
    /// Source IP addresses observed by `AppSync`.
    pub source_ip: Vec<String>,
    /// Caller principal name.
    pub username: String,
    /// Caller ARN.
    pub user_arn: String,
    /// Cognito identity authentication type, when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cognito_identity_auth_type: Option<String>,
    /// Cognito identity authentication provider, when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cognito_identity_auth_provider: Option<String>,
}

/// Amazon `Cognito` user pool identity for AWS `AppSync` Events handlers.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSyncEventsCognitoIdentity {
    /// User pool subject identifier.
    pub sub: String,
    /// Token issuer.
    pub issuer: String,
    /// Authenticated user name.
    pub username: String,
    /// Token claims supplied to the handler.
    pub claims: BTreeMap<String, Value>,
    /// Source IP addresses observed by `AppSync`.
    pub source_ip: Vec<String>,
    /// Default authorization strategy, when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_auth_strategy: Option<String>,
    /// User pool groups for the caller, when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub groups: Option<Vec<String>>,
}

/// `OpenID` Connect identity for AWS `AppSync` Events handlers.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSyncEventsOidcIdentity {
    /// Token claims supplied to the handler.
    pub claims: Value,
    /// Token issuer.
    pub issuer: String,
    /// Subject identifier.
    pub sub: String,
}

/// Caller identity for AWS `AppSync` Events handlers.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum AppSyncEventsIdentity {
    /// AWS Lambda authorizer identity.
    Lambda(AppSyncEventsLambdaIdentity),
    /// Amazon Cognito user pool identity.
    Cognito(AppSyncEventsCognitoIdentity),
    /// IAM identity.
    Iam(AppSyncEventsIamIdentity),
    /// `OpenID` Connect identity.
    Oidc(AppSyncEventsOidcIdentity),
}

/// Incoming event supplied to an AWS `AppSync` Events publish handler.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSyncEventsIncomingEvent {
    /// Event ID supplied by `AppSync`.
    pub id: String,
    /// Event payload supplied by the publisher.
    pub payload: Value,
}

/// AWS `AppSync` Events handler invocation.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSyncEventsEvent {
    /// Caller identity, when the authorization mode supplies one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identity: Option<AppSyncEventsIdentity>,
    /// Data source result available to response handlers, when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    /// Request metadata.
    pub request: AppSyncEventsRequest,
    /// Operation metadata.
    pub info: AppSyncEventsInfo,
    /// Handler error, when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<Value>,
    /// Previous pipeline result, when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prev: Option<Value>,
    /// Handler stash data.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub stash: BTreeMap<String, Value>,
    /// Output errors produced by prior handler work, when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub out_errors: Option<Vec<Value>>,
    /// Published events for `PUBLISH` operations.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub events: Option<Vec<AppSyncEventsIncomingEvent>>,
}

/// Compatibility alias for the AWS `AppSync` Events parser model name.
pub type AppSyncEventsModel = AppSyncEventsEvent;

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::{
        AppSyncEventsEvent, AppSyncEventsIdentity, AppSyncEventsModel, AppSyncEventsOperation,
    };

    #[test]
    fn parses_publish_event_with_lambda_identity() {
        let event = serde_json::from_value::<AppSyncEventsEvent>(json!({
            "identity": {
                "handlerContext": {
                    "tenant": "example"
                }
            },
            "result": null,
            "request": {
                "headers": {
                    "header1": "value1",
                    "multi": ["one", "two"]
                },
                "domainName": "events.example.com"
            },
            "info": {
                "channel": {
                    "path": "/default/foo",
                    "segments": ["default", "foo"]
                },
                "channelNamespace": {
                    "name": "default"
                },
                "operation": "PUBLISH"
            },
            "error": null,
            "prev": null,
            "stash": {},
            "outErrors": [],
            "events": [
                {
                    "payload": {
                        "order_id": "order-1",
                        "quantity": 2
                    },
                    "id": "12345"
                }
            ]
        }))
        .expect("AppSync Events publish event should parse");

        assert_eq!(event.info.operation, AppSyncEventsOperation::Publish);
        assert_eq!(event.info.channel.path, "/default/foo");
        assert_eq!(event.events.as_ref().expect("events")[0].id, "12345");
        assert_eq!(
            event.events.as_ref().expect("events")[0].payload["order_id"].as_str(),
            Some("order-1")
        );

        let Some(AppSyncEventsIdentity::Lambda(identity)) = event.identity else {
            panic!("expected Lambda authorizer identity");
        };
        assert_eq!(identity.handler_context["tenant"], "example");
    }

    #[test]
    fn parses_subscribe_event_without_events() {
        let event = serde_json::from_value::<AppSyncEventsModel>(json!({
            "identity": null,
            "request": {
                "domainName": null
            },
            "info": {
                "channel": {
                    "path": "/default/foo",
                    "segments": ["default", "foo"]
                },
                "channelNamespace": {
                    "name": "default"
                },
                "operation": "SUBSCRIBE"
            },
            "stash": {},
            "events": null
        }))
        .expect("AppSync Events subscribe event should parse");

        assert_eq!(event.identity, None);
        assert_eq!(event.info.operation, AppSyncEventsOperation::Subscribe);
        assert_eq!(event.events, None);
        assert!(event.request.headers.is_empty());
    }

    #[test]
    fn serializes_aws_field_names() {
        let event = serde_json::from_value::<AppSyncEventsEvent>(json!({
            "request": {
                "headers": {
                    "header1": "value1"
                },
                "domainName": null
            },
            "info": {
                "channel": {
                    "path": "/default/foo",
                    "segments": ["default", "foo"]
                },
                "channelNamespace": {
                    "name": "default"
                },
                "operation": "PUBLISH"
            },
            "outErrors": [
                {
                    "message": "warning"
                }
            ],
            "events": [
                {
                    "payload": {
                        "order_id": "order-1"
                    },
                    "id": "12345"
                }
            ]
        }))
        .expect("AppSync Events event should parse");

        let encoded = serde_json::to_value(event).expect("event should serialize");

        assert_eq!(encoded["info"]["operation"], "PUBLISH");
        assert_eq!(encoded["info"]["channelNamespace"]["name"], "default");
        assert_eq!(encoded["outErrors"][0]["message"], "warning");
        assert_eq!(encoded.get("identity"), None);
        assert_eq!(encoded["request"].get("domainName"), None);
    }

    #[test]
    fn parses_cognito_identity_before_oidc_identity() {
        let event = serde_json::from_value::<AppSyncEventsEvent>(json!({
            "identity": {
                "sub": "user-sub",
                "issuer": "https://cognito-idp.us-east-1.amazonaws.com/us-east-1_POOL",
                "username": "user",
                "claims": {
                    "scope": "openid"
                },
                "sourceIp": ["203.0.113.1"],
                "defaultAuthStrategy": null,
                "groups": ["admin"]
            },
            "request": {
                "headers": {},
                "domainName": null
            },
            "info": {
                "channel": {
                    "path": "/default/foo",
                    "segments": ["default", "foo"]
                },
                "channelNamespace": {
                    "name": "default"
                },
                "operation": "SUBSCRIBE"
            },
            "stash": {}
        }))
        .expect("AppSync Events Cognito identity should parse");

        let Some(AppSyncEventsIdentity::Cognito(identity)) = event.identity else {
            panic!("expected Cognito identity");
        };
        assert_eq!(identity.sub, "user-sub");
        assert_eq!(identity.groups, Some(vec!["admin".to_owned()]));
    }

    #[test]
    fn parses_oidc_identity() {
        let event = serde_json::from_value::<AppSyncEventsEvent>(json!({
            "identity": {
                "claims": {
                    "email": "user@example.com"
                },
                "issuer": "https://issuer.example.com",
                "sub": "oidc-sub"
            },
            "request": {
                "headers": {},
                "domainName": null
            },
            "info": {
                "channel": {
                    "path": "/default/foo",
                    "segments": ["default", "foo"]
                },
                "channelNamespace": {
                    "name": "default"
                },
                "operation": "SUBSCRIBE"
            },
            "stash": {}
        }))
        .expect("AppSync Events OIDC identity should parse");

        let Some(AppSyncEventsIdentity::Oidc(identity)) = event.identity else {
            panic!("expected OIDC identity");
        };
        assert_eq!(identity.sub, "oidc-sub");
        assert_eq!(
            identity.claims["email"],
            Value::String("user@example.com".to_owned())
        );
    }
}
