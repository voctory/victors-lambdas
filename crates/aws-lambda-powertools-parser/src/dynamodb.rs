//! Amazon `DynamoDB` event models.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Lambda invocation record for a failed Amazon `DynamoDB` stream batch.
///
/// Lambda sends this record to event source mapping on-failure destinations
/// after a stream batch is discarded.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DynamoDbStreamOnFailureDestination {
    /// Metadata describing the discarded `DynamoDB` stream batch.
    #[serde(rename = "DDBStreamBatchInfo")]
    pub ddb_stream_batch_info: DynamoDbStreamBatchInfo,
    /// Lambda request metadata for the discarded invocation.
    pub request_context: DynamoDbStreamRequestContext,
    /// Lambda response metadata for the discarded invocation.
    pub response_context: DynamoDbStreamResponseContext,
    /// Time Lambda created the destination record.
    pub timestamp: DateTime<Utc>,
    /// Destination record format version.
    pub version: String,
    /// Original invocation payload when Lambda sends the record to Amazon S3.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<String>,
}

/// `DynamoDB` stream batch metadata from a failed Lambda invocation record.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DynamoDbStreamBatchInfo {
    /// Approximate arrival time of the first record in the discarded batch.
    pub approximate_arrival_of_first_record: DateTime<Utc>,
    /// Approximate arrival time of the last record in the discarded batch.
    pub approximate_arrival_of_last_record: DateTime<Utc>,
    /// Number of records in the discarded batch.
    pub batch_size: u64,
    /// Sequence number of the last record in the discarded batch.
    pub end_sequence_number: String,
    /// Shard ID containing the discarded batch.
    pub shard_id: String,
    /// Sequence number of the first record in the discarded batch.
    pub start_sequence_number: String,
    /// `DynamoDB` stream ARN for the discarded batch.
    pub stream_arn: String,
}

/// Lambda request metadata from a failed `DynamoDB` stream invocation record.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DynamoDbStreamRequestContext {
    /// Lambda request ID for the failed invocation.
    pub request_id: String,
    /// ARN of the Lambda function that processed the stream batch.
    pub function_arn: String,
    /// Condition that caused Lambda to discard the batch.
    pub condition: String,
    /// Approximate number of times Lambda invoked the function for this batch.
    pub approximate_invoke_count: u64,
}

/// Lambda response metadata from a failed `DynamoDB` stream invocation record.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DynamoDbStreamResponseContext {
    /// Status code returned by Lambda for the failed invocation.
    pub status_code: u16,
    /// Lambda function version that processed the stream batch.
    pub executed_version: String,
    /// Error category returned by the function.
    pub function_error: String,
}

#[cfg(test)]
mod tests {
    use chrono::DateTime;
    use serde_json::json;

    use super::DynamoDbStreamOnFailureDestination;

    #[test]
    fn parses_dynamodb_stream_on_failure_destination_record() {
        let event = serde_json::from_value::<DynamoDbStreamOnFailureDestination>(json!({
            "requestContext": {
                "requestId": "316aa6d0-8154-xmpl-9af7-85d5f4a6bc81",
                "functionArn": "arn:aws:lambda:us-east-2:123456789012:function:myfunction",
                "condition": "RetryAttemptsExhausted",
                "approximateInvokeCount": 1
            },
            "responseContext": {
                "statusCode": 200,
                "executedVersion": "$LATEST",
                "functionError": "Unhandled"
            },
            "version": "1.0",
            "timestamp": "2019-11-14T00:13:49.717Z",
            "DDBStreamBatchInfo": {
                "shardId": "shardId-00000001573689847184-864758bb",
                "startSequenceNumber": "800000000003126276362",
                "endSequenceNumber": "800000000003126276362",
                "approximateArrivalOfFirstRecord": "2019-11-14T00:13:19Z",
                "approximateArrivalOfLastRecord": "2019-11-14T00:13:19Z",
                "batchSize": 1,
                "streamArn": "arn:aws:dynamodb:us-east-2:123456789012:table/mytable/stream/2019-11-14T00:04:06.388"
            }
        }))
        .expect("DynamoDB stream on-failure destination should parse");

        assert_eq!(event.version, "1.0");
        assert_eq!(
            event.timestamp,
            DateTime::parse_from_rfc3339("2019-11-14T00:13:49.717Z")
                .expect("timestamp should parse")
                .to_utc()
        );
        assert_eq!(
            event.request_context.request_id,
            "316aa6d0-8154-xmpl-9af7-85d5f4a6bc81"
        );
        assert_eq!(
            event.request_context.function_arn,
            "arn:aws:lambda:us-east-2:123456789012:function:myfunction"
        );
        assert_eq!(event.request_context.condition, "RetryAttemptsExhausted");
        assert_eq!(event.request_context.approximate_invoke_count, 1);
        assert_eq!(event.response_context.status_code, 200);
        assert_eq!(event.response_context.executed_version, "$LATEST");
        assert_eq!(event.response_context.function_error, "Unhandled");
        assert_eq!(
            event.ddb_stream_batch_info.shard_id,
            "shardId-00000001573689847184-864758bb"
        );
        assert_eq!(
            event.ddb_stream_batch_info.start_sequence_number,
            "800000000003126276362"
        );
        assert_eq!(
            event.ddb_stream_batch_info.end_sequence_number,
            "800000000003126276362"
        );
        assert_eq!(event.ddb_stream_batch_info.batch_size, 1);
        assert_eq!(
            event.ddb_stream_batch_info.stream_arn,
            "arn:aws:dynamodb:us-east-2:123456789012:table/mytable/stream/2019-11-14T00:04:06.388"
        );
        assert_eq!(event.payload, None);
    }

    #[test]
    fn parses_optional_s3_payload() {
        let event = serde_json::from_value::<DynamoDbStreamOnFailureDestination>(json!({
            "requestContext": {
                "requestId": "316aa6d0-8154-xmpl-9af7-85d5f4a6bc81",
                "functionArn": "arn:aws:lambda:us-east-2:123456789012:function:myfunction",
                "condition": "RetryAttemptsExhausted",
                "approximateInvokeCount": 1
            },
            "responseContext": {
                "statusCode": 200,
                "executedVersion": "$LATEST",
                "functionError": "Unhandled"
            },
            "version": "1.0",
            "timestamp": "2019-11-14T00:13:49.717Z",
            "DDBStreamBatchInfo": {
                "shardId": "shardId-00000001573689847184-864758bb",
                "startSequenceNumber": "800000000003126276362",
                "endSequenceNumber": "800000000003126276362",
                "approximateArrivalOfFirstRecord": "2019-11-14T00:13:19Z",
                "approximateArrivalOfLastRecord": "2019-11-14T00:13:19Z",
                "batchSize": 1,
                "streamArn": "arn:aws:dynamodb:us-east-2:123456789012:table/mytable/stream/2019-11-14T00:04:06.388"
            },
            "payload": "{\"Records\":[]}"
        }))
        .expect("DynamoDB stream S3 destination should parse payload");

        assert_eq!(event.payload.as_deref(), Some("{\"Records\":[]}"));
    }

    #[test]
    fn serializes_with_aws_field_names() {
        let event = serde_json::from_value::<DynamoDbStreamOnFailureDestination>(json!({
            "requestContext": {
                "requestId": "316aa6d0-8154-xmpl-9af7-85d5f4a6bc81",
                "functionArn": "arn:aws:lambda:us-east-2:123456789012:function:myfunction",
                "condition": "RetryAttemptsExhausted",
                "approximateInvokeCount": 1
            },
            "responseContext": {
                "statusCode": 200,
                "executedVersion": "$LATEST",
                "functionError": "Unhandled"
            },
            "version": "1.0",
            "timestamp": "2019-11-14T00:13:49.717Z",
            "DDBStreamBatchInfo": {
                "shardId": "shardId-00000001573689847184-864758bb",
                "startSequenceNumber": "800000000003126276362",
                "endSequenceNumber": "800000000003126276362",
                "approximateArrivalOfFirstRecord": "2019-11-14T00:13:19Z",
                "approximateArrivalOfLastRecord": "2019-11-14T00:13:19Z",
                "batchSize": 1,
                "streamArn": "arn:aws:dynamodb:us-east-2:123456789012:table/mytable/stream/2019-11-14T00:04:06.388"
            }
        }))
        .expect("DynamoDB stream on-failure destination should parse");

        let encoded = serde_json::to_value(event).expect("destination should serialize");

        assert_eq!(
            encoded["DDBStreamBatchInfo"]["streamArn"],
            "arn:aws:dynamodb:us-east-2:123456789012:table/mytable/stream/2019-11-14T00:04:06.388"
        );
        assert_eq!(
            encoded["requestContext"]["functionArn"],
            "arn:aws:lambda:us-east-2:123456789012:function:myfunction"
        );
        assert_eq!(encoded.get("payload"), None);
    }
}
