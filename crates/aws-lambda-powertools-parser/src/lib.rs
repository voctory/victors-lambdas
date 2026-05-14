//! Event parsing utility.

#[cfg(feature = "aws-lambda-events")]
mod appsync;
mod appsync_events;
mod bedrock_agent;
mod cognito;
mod dynamodb;
#[cfg(feature = "aws-lambda-events")]
mod envelope;
mod error;
mod iot_registry;
mod parser;
mod s3_eventbridge;
mod transfer_family;

#[cfg(feature = "aws-lambda-events")]
pub use appsync::{AppSyncBatchResolverEvent, AppSyncResolverEvent};
pub use appsync_events::{
    AppSyncEventsChannel, AppSyncEventsChannelNamespace, AppSyncEventsCognitoIdentity,
    AppSyncEventsEvent, AppSyncEventsIamIdentity, AppSyncEventsIdentity,
    AppSyncEventsIncomingEvent, AppSyncEventsInfo, AppSyncEventsLambdaIdentity, AppSyncEventsModel,
    AppSyncEventsOidcIdentity, AppSyncEventsOperation, AppSyncEventsRequest,
};
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::apigw::ApiGatewayCustomAuthorizerRequest as ApiGatewayAuthorizerToken;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::apigw::ApiGatewayCustomAuthorizerRequestTypeRequest as ApiGatewayAuthorizerRequest;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::apigw::ApiGatewayCustomAuthorizerResponse as ApiGatewayAuthorizerResponse;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::apigw::ApiGatewayV2CustomAuthorizerIamPolicyResponse as ApiGatewayAuthorizerIamPolicyResponse;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::apigw::ApiGatewayV2CustomAuthorizerSimpleResponse as ApiGatewayAuthorizerSimpleResponse;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::apigw::ApiGatewayV2CustomAuthorizerV1Request as ApiGatewayAuthorizerHttpApiV1Request;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::apigw::ApiGatewayV2CustomAuthorizerV2Request as ApiGatewayAuthorizerRequestV2;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::apigw::ApiGatewayWebsocketProxyRequest as ApiGatewayWebsocketConnectEvent;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::apigw::ApiGatewayWebsocketProxyRequest as ApiGatewayWebsocketDisconnectEvent;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::apigw::ApiGatewayWebsocketProxyRequest as ApiGatewayWebsocketMessageEvent;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::cloudformation::CloudFormationCustomResourceRequest;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::cloudformation::CloudFormationCustomResourceResponse;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::cloudformation::CloudFormationCustomResourceResponseStatus;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::cloudformation::CreateRequest as CloudFormationCustomResourceCreate;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::cloudformation::DeleteRequest as CloudFormationCustomResourceDelete;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::cloudformation::UpdateRequest as CloudFormationCustomResourceUpdate;
pub use bedrock_agent::{
    BedrockAgentEvent, BedrockAgentEventModel, BedrockAgentFunctionAgent,
    BedrockAgentFunctionEvent, BedrockAgentFunctionEventModel, BedrockAgentFunctionParameter,
    BedrockAgentModel, BedrockAgentPropertyModel, BedrockAgentRequestBody,
    BedrockAgentRequestBodyModel, BedrockAgentRequestMedia, BedrockAgentRequestMediaModel,
};
pub use cognito::{
    CognitoCustomEmailSenderTriggerEvent, CognitoCustomEmailSenderTriggerModel,
    CognitoCustomEmailSenderTriggerSource, CognitoCustomSMSSenderTriggerModel,
    CognitoCustomSenderRequest, CognitoCustomSenderRequestType, CognitoCustomSmsSenderTriggerEvent,
    CognitoCustomSmsSenderTriggerSource, CognitoMigrateUserRequest, CognitoMigrateUserResponse,
    CognitoMigrateUserTriggerEvent, CognitoMigrateUserTriggerModel,
    CognitoMigrateUserTriggerSource, CognitoUserPoolCallerContext,
};
pub use dynamodb::{
    DynamoDbStreamBatchInfo, DynamoDbStreamOnFailureDestination, DynamoDbStreamRequestContext,
    DynamoDbStreamResponseContext,
};
pub use error::{ParseError, ParseErrorKind};
pub use iot_registry::{
    IoTCoreAddOrDeleteFromThingGroupEvent, IoTCoreAddOrRemoveFromThingGroupEvent,
    IoTCorePropagatingAttribute, IoTCoreRegistryCrudOperation, IoTCoreRegistryEventType,
    IoTCoreRegistryMembershipOperation, IoTCoreThingEvent, IoTCoreThingGroupEvent,
    IoTCoreThingGroupHierarchyEvent, IoTCoreThingGroupMembershipEvent, IoTCoreThingGroupReference,
    IoTCoreThingTypeAssociationEvent, IoTCoreThingTypeEvent,
};
pub use parser::{EventParser, ParsedEvent};
pub use s3_eventbridge::{
    S3EventBridgeBucket, S3EventBridgeDetail, S3EventBridgeEvent, S3EventBridgeObject,
    S3EventNotificationEventBridgeDetailModel, S3EventNotificationEventBridgeModel,
};
pub use transfer_family::{
    TransferFamilyAuthorizerEvent, TransferFamilyAuthorizerResponse,
    TransferFamilyHomeDirectoryEntry, TransferFamilyHomeDirectoryType, TransferFamilyPosixProfile,
    TransferFamilyProtocol, TransferFamilyResponseError, TransferFamilyResponseResult,
};
