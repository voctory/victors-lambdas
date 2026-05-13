//! Amazon `Cognito` user pool event models.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Common caller context for Amazon `Cognito` user pool trigger events.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CognitoUserPoolCallerContext {
    /// AWS SDK version used by Amazon `Cognito`.
    #[serde(rename = "awsSdkVersion")]
    pub aws_sdk_version: String,
    /// App client ID associated with the request.
    pub client_id: String,
}

/// Trigger source for Amazon `Cognito` custom email sender events.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub enum CognitoCustomEmailSenderTriggerSource {
    /// User signs up and receives a welcome message.
    #[serde(rename = "CustomEmailSender_SignUp")]
    SignUp,
    /// User signs in and receives an email OTP or MFA code.
    #[serde(rename = "CustomEmailSender_Authentication")]
    Authentication,
    /// User requests a password reset code.
    #[serde(rename = "CustomEmailSender_ForgotPassword")]
    ForgotPassword,
    /// User requests a replacement account-confirmation code.
    #[serde(rename = "CustomEmailSender_ResendCode")]
    ResendCode,
    /// User updates an attribute and receives a verification code.
    #[serde(rename = "CustomEmailSender_UpdateUserAttribute")]
    UpdateUserAttribute,
    /// User creates an attribute and receives a verification code.
    #[serde(rename = "CustomEmailSender_VerifyUserAttribute")]
    VerifyUserAttribute,
    /// Administrator creates a user and sends them a temporary password.
    #[serde(rename = "CustomEmailSender_AdminCreateUser")]
    AdminCreateUser,
    /// Threat protection sends an account takeover notification.
    #[serde(rename = "CustomEmailSender_AccountTakeOverNotification")]
    AccountTakeOverNotification,
}

/// Trigger source for Amazon `Cognito` custom SMS sender events.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub enum CognitoCustomSmsSenderTriggerSource {
    /// User signs up and receives a welcome message.
    #[serde(rename = "CustomSMSSender_SignUp", alias = "CustomSmsSender_SignUp")]
    SignUp,
    /// User signs in and receives an SMS OTP or MFA code.
    #[serde(
        rename = "CustomSMSSender_Authentication",
        alias = "CustomSmsSender_Authentication"
    )]
    Authentication,
    /// User requests a password reset code.
    #[serde(
        rename = "CustomSMSSender_ForgotPassword",
        alias = "CustomSmsSender_ForgotPassword"
    )]
    ForgotPassword,
    /// User requests a replacement account-confirmation code.
    #[serde(
        rename = "CustomSMSSender_ResendCode",
        alias = "CustomSmsSender_ResendCode"
    )]
    ResendCode,
    /// User updates an attribute and receives a verification code.
    #[serde(
        rename = "CustomSMSSender_UpdateUserAttribute",
        alias = "CustomSmsSender_UpdateUserAttribute"
    )]
    UpdateUserAttribute,
    /// User creates an attribute and receives a verification code.
    #[serde(
        rename = "CustomSMSSender_VerifyUserAttribute",
        alias = "CustomSmsSender_VerifyUserAttribute"
    )]
    VerifyUserAttribute,
    /// Administrator creates a user and sends them a temporary password.
    #[serde(
        rename = "CustomSMSSender_AdminCreateUser",
        alias = "CustomSmsSender_AdminCreateUser"
    )]
    AdminCreateUser,
}

/// Request version for Amazon `Cognito` custom sender trigger events.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub enum CognitoCustomSenderRequestType {
    /// Custom email sender request version 1.
    #[serde(rename = "customEmailSenderRequestV1")]
    CustomEmailSenderRequestV1,
    /// Custom SMS sender request version 1.
    #[serde(rename = "customSMSSenderRequestV1")]
    CustomSmsSenderRequestV1,
}

/// Request payload shared by Amazon `Cognito` custom email and SMS sender events.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CognitoCustomSenderRequest {
    /// Custom sender request version.
    #[serde(rename = "type")]
    pub request_type: CognitoCustomSenderRequestType,
    /// Encrypted one-time code that the function decrypts and sends.
    pub code: String,
    /// Client metadata passed through supported Amazon `Cognito` API calls.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub client_metadata: BTreeMap<String, String>,
    /// User attributes included in the custom sender event.
    pub user_attributes: BTreeMap<String, Value>,
}

/// Amazon `Cognito` custom email sender trigger event.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CognitoCustomEmailSenderTriggerEvent {
    /// Event schema version.
    pub version: String,
    /// Trigger source that caused Amazon `Cognito` to send an email.
    pub trigger_source: CognitoCustomEmailSenderTriggerSource,
    /// AWS Region of the user pool.
    pub region: String,
    /// User pool ID.
    pub user_pool_id: String,
    /// User name associated with the event.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_name: Option<String>,
    /// Caller context supplied by Amazon `Cognito`.
    pub caller_context: CognitoUserPoolCallerContext,
    /// Custom sender request payload.
    pub request: CognitoCustomSenderRequest,
}

/// Compatibility alias for the custom email sender trigger model name.
pub type CognitoCustomEmailSenderTriggerModel = CognitoCustomEmailSenderTriggerEvent;

/// Amazon `Cognito` custom SMS sender trigger event.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CognitoCustomSmsSenderTriggerEvent {
    /// Event schema version.
    pub version: String,
    /// Trigger source that caused Amazon `Cognito` to send an SMS message.
    pub trigger_source: CognitoCustomSmsSenderTriggerSource,
    /// AWS Region of the user pool.
    pub region: String,
    /// User pool ID.
    pub user_pool_id: String,
    /// User name associated with the event.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_name: Option<String>,
    /// Caller context supplied by Amazon `Cognito`.
    pub caller_context: CognitoUserPoolCallerContext,
    /// Custom sender request payload.
    pub request: CognitoCustomSenderRequest,
}

/// Compatibility alias for the custom SMS sender trigger model name.
pub type CognitoCustomSMSSenderTriggerModel = CognitoCustomSmsSenderTriggerEvent;

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        CognitoCustomEmailSenderTriggerEvent, CognitoCustomEmailSenderTriggerSource,
        CognitoCustomSMSSenderTriggerModel, CognitoCustomSenderRequestType,
        CognitoCustomSmsSenderTriggerEvent, CognitoCustomSmsSenderTriggerSource,
    };

    #[test]
    fn parses_custom_email_sender_event() {
        let event = serde_json::from_value::<CognitoCustomEmailSenderTriggerEvent>(json!({
            "version": "1",
            "triggerSource": "CustomEmailSender_SignUp",
            "region": "us-east-1",
            "userPoolId": "us-east-1_ABC123",
            "userName": "johndoe",
            "callerContext": {
                "awsSdkVersion": "2.814.0",
                "clientId": "client123"
            },
            "request": {
                "type": "customEmailSenderRequestV1",
                "code": "encrypted-code",
                "clientMetadata": {
                    "campaign": "welcome"
                },
                "userAttributes": {
                    "email": "user@example.com",
                    "email_verified": true
                }
            }
        }))
        .expect("custom email sender event should parse");

        assert_eq!(
            event.trigger_source,
            CognitoCustomEmailSenderTriggerSource::SignUp
        );
        assert_eq!(event.user_name.as_deref(), Some("johndoe"));
        assert_eq!(event.caller_context.aws_sdk_version, "2.814.0");
        assert_eq!(
            event.request.request_type,
            CognitoCustomSenderRequestType::CustomEmailSenderRequestV1
        );
        assert_eq!(event.request.client_metadata["campaign"], "welcome");
        assert_eq!(event.request.user_attributes["email"], "user@example.com");
        assert_eq!(event.request.user_attributes["email_verified"], true);
    }

    #[test]
    fn parses_custom_sms_sender_event() {
        let event = serde_json::from_value::<CognitoCustomSmsSenderTriggerEvent>(json!({
            "version": "1",
            "triggerSource": "CustomSMSSender_Authentication",
            "region": "us-east-1",
            "userPoolId": "us-east-1_ABC123",
            "callerContext": {
                "awsSdkVersion": "2.814.0",
                "clientId": "client123"
            },
            "request": {
                "type": "customSMSSenderRequestV1",
                "code": "encrypted-code",
                "userAttributes": {
                    "phone_number": "+15555550100",
                    "phone_number_verified": false
                }
            }
        }))
        .expect("custom SMS sender event should parse");

        assert_eq!(
            event.trigger_source,
            CognitoCustomSmsSenderTriggerSource::Authentication
        );
        assert_eq!(event.user_name, None);
        assert_eq!(
            event.request.request_type,
            CognitoCustomSenderRequestType::CustomSmsSenderRequestV1
        );
        assert_eq!(
            event.request.user_attributes["phone_number"],
            "+15555550100"
        );
        assert_eq!(
            event.request.user_attributes["phone_number_verified"],
            false
        );
    }

    #[test]
    fn parses_documented_sms_source_casing_alias() {
        let event = serde_json::from_value::<CognitoCustomSMSSenderTriggerModel>(json!({
            "version": "1",
            "triggerSource": "CustomSmsSender_ForgotPassword",
            "region": "us-east-1",
            "userPoolId": "us-east-1_ABC123",
            "callerContext": {
                "awsSdkVersion": "2.814.0",
                "clientId": "client123"
            },
            "request": {
                "type": "customSMSSenderRequestV1",
                "code": "encrypted-code",
                "userAttributes": {
                    "phone_number": "+15555550100"
                }
            }
        }))
        .expect("custom SMS sender source alias should parse");

        assert_eq!(
            event.trigger_source,
            CognitoCustomSmsSenderTriggerSource::ForgotPassword
        );
    }

    #[test]
    fn serializes_custom_sender_field_names() {
        let event = serde_json::from_value::<CognitoCustomEmailSenderTriggerEvent>(json!({
            "version": "1",
            "triggerSource": "CustomEmailSender_AccountTakeOverNotification",
            "region": "us-east-1",
            "userPoolId": "us-east-1_ABC123",
            "callerContext": {
                "awsSdkVersion": "2.814.0",
                "clientId": "client123"
            },
            "request": {
                "type": "customEmailSenderRequestV1",
                "code": "encrypted-code",
                "userAttributes": {
                    "email": "user@example.com"
                }
            }
        }))
        .expect("custom email sender event should parse");

        let encoded = serde_json::to_value(event).expect("event should serialize");

        assert_eq!(
            encoded["triggerSource"],
            "CustomEmailSender_AccountTakeOverNotification"
        );
        assert_eq!(encoded["callerContext"]["awsSdkVersion"], "2.814.0");
        assert_eq!(encoded["request"]["type"], "customEmailSenderRequestV1");
        assert!(encoded["request"].get("clientMetadata").is_none());
    }
}
