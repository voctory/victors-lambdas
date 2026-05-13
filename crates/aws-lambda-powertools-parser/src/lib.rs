//! Event parsing utility.

mod cognito;
mod dynamodb;
#[cfg(feature = "aws-lambda-events")]
mod envelope;
mod error;
mod iot_registry;
mod parser;
mod transfer_family;

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
pub use transfer_family::{
    TransferFamilyAuthorizerEvent, TransferFamilyAuthorizerResponse,
    TransferFamilyHomeDirectoryEntry, TransferFamilyHomeDirectoryType, TransferFamilyPosixProfile,
    TransferFamilyProtocol, TransferFamilyResponseError, TransferFamilyResponseResult,
};
