//! Common Powertools `JMESPath` envelope expressions.

use serde::{Serialize, de::DeserializeOwned};

use crate::{JmespathResult, search_as};

/// API Gateway REST API body JSON envelope.
pub const API_GATEWAY_REST: &str = "powertools_json(body)";

/// API Gateway HTTP API body JSON envelope.
pub const API_GATEWAY_HTTP: &str = API_GATEWAY_REST;

/// SQS message body JSON envelope.
pub const SQS: &str = "Records[*].powertools_json(body)";

/// SNS message JSON envelope.
pub const SNS: &str = "Records[0].Sns.Message | powertools_json(@)";

/// `EventBridge` detail envelope.
pub const EVENTBRIDGE: &str = "detail";

/// Scheduled `EventBridge` detail envelope.
pub const CLOUDWATCH_EVENTS_SCHEDULED: &str = EVENTBRIDGE;

/// Kinesis data stream base64 JSON record envelope.
pub const KINESIS_DATA_STREAM: &str = "Records[*].kinesis.powertools_json(powertools_base64(data))";

/// `CloudWatch Logs` base64-gzip JSON event envelope.
pub const CLOUDWATCH_LOGS: &str =
    "awslogs.powertools_base64_gzip(data) | powertools_json(@).logEvents[*]";

/// S3 event delivered through SNS and SQS envelope.
pub const S3_SNS_SQS: &str = "Records[*].powertools_json(body).powertools_json(Message).Records[0]";

/// S3 event delivered through SQS envelope.
pub const S3_SQS: &str = "Records[*].powertools_json(body).Records[0]";

/// S3 event delivered through SNS and Kinesis Firehose envelope.
pub const S3_SNS_KINESIS_FIREHOSE: &str =
    "records[*].powertools_json(powertools_base64(data)).powertools_json(Message).Records[0]";

/// S3 event delivered through Kinesis Firehose envelope.
pub const S3_KINESIS_FIREHOSE: &str =
    "records[*].powertools_json(powertools_base64(data)).Records[0]";

/// S3 `EventBridge` event delivered through SQS envelope.
pub const S3_EVENTBRIDGE_SQS: &str = "Records[*].powertools_json(body).detail";

/// Extracts data from an envelope expression and decodes it into `T`.
///
/// The expression can use standard `JMESPath` functions plus Powertools helpers:
/// `powertools_json`, `powertools_base64`, and `powertools_base64_gzip`.
///
/// # Errors
///
/// Returns an error when the envelope expression cannot be compiled or
/// evaluated, or when the selected value cannot be decoded into `T`.
pub fn extract_data_from_envelope<T, D>(data: D, envelope: &str) -> JmespathResult<T>
where
    T: DeserializeOwned,
    D: Serialize,
{
    search_as(envelope, data)
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;
    use serde_json::json;

    use super::{API_GATEWAY_REST, SQS, extract_data_from_envelope};

    #[derive(Debug, Deserialize, Eq, PartialEq)]
    struct Order {
        order_id: String,
        quantity: u32,
    }

    #[test]
    fn extracts_api_gateway_body() {
        let order = extract_data_from_envelope::<Order, _>(
            json!({
                "body": "{\"order_id\":\"order-1\",\"quantity\":2}"
            }),
            API_GATEWAY_REST,
        )
        .expect("API Gateway body should extract");

        assert_eq!(order.order_id, "order-1");
        assert_eq!(order.quantity, 2);
    }

    #[test]
    fn extracts_sqs_bodies() {
        let orders = extract_data_from_envelope::<Vec<Order>, _>(
            json!({
                "Records": [
                    {
                        "body": "{\"order_id\":\"order-1\",\"quantity\":2}"
                    },
                    {
                        "body": "{\"order_id\":\"order-2\",\"quantity\":3}"
                    }
                ]
            }),
            SQS,
        )
        .expect("SQS bodies should extract");

        assert_eq!(orders.len(), 2);
        assert_eq!(orders[0].order_id, "order-1");
        assert_eq!(orders[1].quantity, 3);
    }
}
