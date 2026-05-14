//! Event parsing utility.

#[cfg(feature = "aws-lambda-events")]
mod appsync;
mod appsync_events;
mod bedrock_agent;
mod cloudwatch_custom_widget;
mod cognito;
mod dynamodb;
#[cfg(feature = "aws-lambda-events")]
mod envelope;
mod error;
mod iot_registry;
mod parser;
mod s3_eventbridge;
mod s3_notification;
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
pub use aws_lambda_events::event::activemq::ActiveMqEvent as ActiveMqModel;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::alb::AlbTargetGroupRequest as AlbModel;
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
pub use aws_lambda_events::event::apigw::ApiGatewayProxyRequest as ApiGatewayProxyEventModel;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::apigw::ApiGatewayProxyRequestContext as ApiGatewayProxyRequestContextModel;
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
pub use aws_lambda_events::event::apigw::ApiGatewayV2httpRequest as ApiGatewayProxyEventV2Model;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::apigw::ApiGatewayV2httpRequestContext as ApiGatewayProxyEventV2RequestContextModel;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::apigw::ApiGatewayWebsocketProxyRequest as ApiGatewayWebsocketConnectEvent;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::apigw::ApiGatewayWebsocketProxyRequest as ApiGatewayWebsocketDisconnectEvent;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::apigw::ApiGatewayWebsocketProxyRequest as ApiGatewayWebsocketEventModel;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::apigw::ApiGatewayWebsocketProxyRequest as ApiGatewayWebsocketMessageEvent;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::appsync::AppSyncLambdaAuthorizerRequest as AppSyncAuthorizerEvent;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::appsync::AppSyncLambdaAuthorizerResponse as AppSyncAuthorizerResponse;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::autoscaling::AutoScalingEvent as AutoScalingModel;
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
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::cloudwatch_alarms::CloudWatchAlarm as CloudWatchAlarmModel;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::cloudwatch_alarms::CloudWatchCompositeAlarm as CloudWatchCompositeAlarmModel;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::cloudwatch_alarms::CloudWatchMetricAlarm as CloudWatchMetricAlarmModel;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::cloudwatch_logs::LogData as CloudWatchLogsDecodeModel;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::cloudwatch_logs::LogEntry as CloudWatchLogEventModel;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::cloudwatch_logs::LogsEvent as CloudWatchLogsModel;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::code_commit::CodeCommitEvent as CodeCommitModel;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::codedeploy::CodeDeployLifecycleEvent as CodeDeployLifecycleHookEventModel;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::codepipeline_job::CodePipelineJobEvent as CodePipelineJobEventModel;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::cognito::CognitoEventUserPoolsCreateAuthChallenge as CognitoCreateAuthChallengeTriggerModel;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::cognito::CognitoEventUserPoolsCustomMessage as CognitoCustomMessageTriggerModel;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::cognito::CognitoEventUserPoolsDefineAuthChallenge as CognitoDefineAuthChallengeTriggerModel;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::cognito::CognitoEventUserPoolsPostAuthentication as CognitoPostAuthenticationTriggerModel;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::cognito::CognitoEventUserPoolsPostConfirmation as CognitoPostConfirmationTriggerModel;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::cognito::CognitoEventUserPoolsPreAuthentication as CognitoPreAuthenticationTriggerModel;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::cognito::CognitoEventUserPoolsPreSignup as CognitoPreSignupTriggerModel;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::cognito::CognitoEventUserPoolsPreTokenGen as CognitoPreTokenGenerationTriggerModelV1;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::cognito::CognitoEventUserPoolsPreTokenGenV2 as CognitoPreTokenGenerationTriggerModelV2AndV3;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::cognito::CognitoEventUserPoolsVerifyAuthChallenge as CognitoVerifyAuthChallengeTriggerModel;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::config::ConfigEvent as AwsConfigModel;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::connect::ConnectEvent as ConnectContactFlowEventModel;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::dynamodb::Event as DynamoDbStreamModel;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::dynamodb::EventRecord as DynamoDbStreamRecordModel;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::eventbridge::EventBridgeEvent as EventBridgeModel;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::firehose::KinesisFirehoseEvent as KinesisFirehoseModel;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::firehose::KinesisFirehoseEvent as KinesisFirehoseSqsModel;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::firehose::KinesisFirehoseEventRecord as KinesisFirehoseRecordModel;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::firehose::KinesisFirehoseEventRecord as KinesisFirehoseSqsRecordModel;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::kafka::KafkaEvent as KafkaMskEventModel;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::kafka::KafkaEvent as KafkaSelfManagedEventModel;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::kafka::KafkaRecord as KafkaRecordModel;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::kinesis::KinesisEvent as KinesisDataStreamModel;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::kinesis::KinesisEvent as KinesisDynamoDbStreamModel;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::kinesis::KinesisEventRecord as KinesisDataStreamRecordModel;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::lambda_function_urls::LambdaFunctionUrlRequest as LambdaFunctionUrlModel;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::rabbitmq::RabbitMqEvent as RabbitMqModel;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::s3::S3Event as S3Model;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::s3::batch_job::S3BatchJobEvent as S3BatchOperationModel;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::s3::object_lambda::S3ObjectLambdaEvent;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::s3::object_lambda::S3ObjectLambdaEvent as S3ObjectLambdaModel;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::secretsmanager::SecretsManagerSecretRotationEvent as SecretsManagerRotationEventModel;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::ses::SimpleEmailEvent as SesModel;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::ses::SimpleEmailRecord as SesRecordModel;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::sns::SnsEvent as SnsModel;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::sns::SnsMessage as SnsNotificationModel;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::sns::SnsMessage as SnsSqsNotificationModel;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::sns::SnsRecord as SnsRecordModel;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::sqs::SqsEvent as S3SqsEventNotificationModel;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::sqs::SqsEvent as SqsModel;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::sqs::SqsMessage as SqsRecordModel;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::sqs::SqsMessageAttribute as SqsMessageAttributeModel;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::vpc_lattice::VpcLatticeRequestV1 as VpcLatticeModel;
#[cfg(feature = "aws-lambda-events")]
#[doc(inline)]
pub use aws_lambda_events::event::vpc_lattice::VpcLatticeRequestV2 as VpcLatticeV2Model;
pub use bedrock_agent::{
    BedrockAgentEvent, BedrockAgentEventModel, BedrockAgentFunctionAgent,
    BedrockAgentFunctionEvent, BedrockAgentFunctionEventModel, BedrockAgentFunctionParameter,
    BedrockAgentModel, BedrockAgentPropertyModel, BedrockAgentRequestBody,
    BedrockAgentRequestBodyModel, BedrockAgentRequestMedia, BedrockAgentRequestMediaModel,
};
pub use cloudwatch_custom_widget::{
    CloudWatchDashboardCustomWidgetEvent, CloudWatchDashboardCustomWidgetModel,
    CloudWatchWidgetContext, CloudWatchWidgetForms, CloudWatchWidgetTimeRange,
    CloudWatchWidgetTimeZone, CloudWatchWidgetZoom,
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
    DynamoDbStreamBatchInfo, DynamoDbStreamImageRecord, DynamoDbStreamOnFailureDestination,
    DynamoDbStreamRequestContext, DynamoDbStreamResponseContext,
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
pub use s3_notification::{
    S3EventNotification, S3EventNotificationBucket, S3EventNotificationEntity,
    S3EventNotificationGlacierEventData, S3EventNotificationGlacierRestoreEventData,
    S3EventNotificationIdentity, S3EventNotificationIntelligentTieringEventData,
    S3EventNotificationModel, S3EventNotificationObject, S3EventNotificationRecord,
    S3EventNotificationRecordModel, S3EventNotificationRequestParameters,
    S3EventNotificationResponseElements, S3EventRecordIntelligentTieringEventData, S3RecordModel,
};
pub use transfer_family::{
    TransferFamilyAuthorizerEvent, TransferFamilyAuthorizerResponse,
    TransferFamilyHomeDirectoryEntry, TransferFamilyHomeDirectoryType, TransferFamilyPosixProfile,
    TransferFamilyProtocol, TransferFamilyResponseError, TransferFamilyResponseResult,
};

#[cfg(all(test, feature = "aws-lambda-events"))]
mod tests {
    use super::{
        ApiGatewayProxyEventV2RequestContextModel, ApiGatewayProxyRequestContextModel,
        ApiGatewayWebsocketEventModel, CloudWatchLogEventModel, CloudWatchLogsDecodeModel,
        DynamoDbStreamRecordModel, KafkaRecordModel, KinesisDataStreamRecordModel,
        KinesisDynamoDbStreamModel, KinesisFirehoseRecordModel, KinesisFirehoseSqsRecordModel,
        S3ObjectLambdaModel, SesRecordModel, SnsNotificationModel, SnsRecordModel,
        SnsSqsNotificationModel, SqsMessageAttributeModel, SqsRecordModel,
    };

    fn assert_public_alias<T>() {}

    #[test]
    fn exposes_record_level_parser_aliases() {
        assert_public_alias::<ApiGatewayProxyRequestContextModel>();
        assert_public_alias::<ApiGatewayProxyEventV2RequestContextModel>();
        assert_public_alias::<ApiGatewayWebsocketEventModel>();
        assert_public_alias::<CloudWatchLogEventModel>();
        assert_public_alias::<CloudWatchLogsDecodeModel>();
        assert_public_alias::<DynamoDbStreamRecordModel>();
        assert_public_alias::<KafkaRecordModel>();
        assert_public_alias::<KinesisDataStreamRecordModel>();
        assert_public_alias::<KinesisDynamoDbStreamModel>();
        assert_public_alias::<KinesisFirehoseRecordModel>();
        assert_public_alias::<KinesisFirehoseSqsRecordModel>();
        assert_public_alias::<S3ObjectLambdaModel>();
        assert_public_alias::<SesRecordModel>();
        assert_public_alias::<SnsNotificationModel>();
        assert_public_alias::<SnsRecordModel>();
        assert_public_alias::<SnsSqsNotificationModel>();
        assert_public_alias::<SqsMessageAttributeModel>();
        assert_public_alias::<SqsRecordModel>();
    }
}
