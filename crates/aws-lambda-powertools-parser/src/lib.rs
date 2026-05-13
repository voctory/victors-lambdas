//! Event parsing utility.

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

pub use appsync_events::{
    AppSyncEventsChannel, AppSyncEventsChannelNamespace, AppSyncEventsCognitoIdentity,
    AppSyncEventsEvent, AppSyncEventsIamIdentity, AppSyncEventsIdentity,
    AppSyncEventsIncomingEvent, AppSyncEventsInfo, AppSyncEventsLambdaIdentity, AppSyncEventsModel,
    AppSyncEventsOidcIdentity, AppSyncEventsOperation, AppSyncEventsRequest,
};
pub use bedrock_agent::{
    BedrockAgentFunctionAgent, BedrockAgentFunctionEvent, BedrockAgentFunctionEventModel,
    BedrockAgentFunctionParameter,
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
