//! Amazon S3 event notification models.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Amazon S3 event notification.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct S3EventNotification {
    /// Records included in the S3 event notification.
    #[serde(rename = "Records")]
    pub records: Vec<S3EventNotificationRecord>,
}

/// Amazon S3 event notification record.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct S3EventNotificationRecord {
    /// Version of the event message.
    pub event_version: String,
    /// Event source, normally `aws:s3`.
    pub event_source: String,
    /// AWS Region where the event occurred.
    pub aws_region: String,
    /// Time when Amazon S3 finished processing the request.
    pub event_time: DateTime<Utc>,
    /// S3 event notification type without the `s3:` prefix.
    pub event_name: String,
    /// Principal that caused the event.
    pub user_identity: S3EventNotificationIdentity,
    /// S3 request parameters.
    pub request_parameters: S3EventNotificationRequestParameters,
    /// S3 response elements.
    pub response_elements: S3EventNotificationResponseElements,
    /// S3 bucket and object metadata.
    pub s3: S3EventNotificationEntity,
    /// Glacier restore event data, when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub glacier_event_data: Option<S3EventNotificationGlacierEventData>,
    /// Intelligent-Tiering event data, when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intelligent_tiering_event_data: Option<S3EventNotificationIntelligentTieringEventData>,
}

/// Amazon S3 identity metadata.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct S3EventNotificationIdentity {
    /// Principal identifier.
    pub principal_id: String,
}

/// Amazon S3 request parameters.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct S3EventNotificationRequestParameters {
    /// Source IP address or service principal of the request.
    #[serde(rename = "sourceIPAddress")]
    pub source_ip_address: String,
}

/// Amazon S3 response elements.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct S3EventNotificationResponseElements {
    /// Amazon S3 generated request ID.
    #[serde(rename = "x-amz-request-id")]
    pub x_amz_request_id: String,
    /// Amazon S3 host ID.
    #[serde(rename = "x-amz-id-2")]
    pub x_amz_id_2: String,
    /// Additional response elements.
    #[serde(flatten)]
    pub additional: HashMap<String, String>,
}

/// Amazon S3 bucket and object metadata in an event notification record.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct S3EventNotificationEntity {
    /// S3 notification schema version.
    #[serde(rename = "s3SchemaVersion")]
    pub schema_version: String,
    /// Bucket notification configuration ID.
    pub configuration_id: String,
    /// Bucket associated with the S3 event.
    pub bucket: S3EventNotificationBucket,
    /// Object associated with most S3 event types.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub object: Option<S3EventNotificationObject>,
    /// Object associated with S3 Intelligent-Tiering events.
    #[serde(
        default,
        rename = "get_object",
        skip_serializing_if = "Option::is_none"
    )]
    pub get_object: Option<S3EventNotificationObject>,
}

impl S3EventNotificationEntity {
    /// Returns the object metadata regardless of whether the event used
    /// `object` or Intelligent-Tiering's `get_object` field.
    pub fn object(&self) -> Option<&S3EventNotificationObject> {
        self.object.as_ref().or(self.get_object.as_ref())
    }
}

/// Amazon S3 bucket metadata in an event notification record.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct S3EventNotificationBucket {
    /// Bucket name.
    pub name: String,
    /// Bucket owner identity.
    pub owner_identity: S3EventNotificationIdentity,
    /// Bucket ARN.
    pub arn: String,
}

/// Amazon S3 object metadata in an event notification record.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct S3EventNotificationObject {
    /// Object key.
    pub key: String,
    /// Object size in bytes, when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    /// URL-decoded object key, when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url_decoded_key: Option<String>,
    /// Object entity tag, when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub e_tag: Option<String>,
    /// Object sequencer, when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sequencer: Option<String>,
    /// Object version ID, when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version_id: Option<String>,
}

/// Glacier restore event data in an S3 event notification record.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct S3EventNotificationGlacierEventData {
    /// Restore event data.
    pub restore_event_data: S3EventNotificationGlacierRestoreEventData,
}

/// Glacier restore detail in an S3 event notification record.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct S3EventNotificationGlacierRestoreEventData {
    /// Restore expiry time.
    pub lifecycle_restoration_expiry_time: DateTime<Utc>,
    /// Source storage class for the restore.
    pub lifecycle_restore_storage_class: String,
}

/// Intelligent-Tiering event data in an S3 event notification record.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct S3EventNotificationIntelligentTieringEventData {
    /// New access tier for the object.
    pub destination_access_tier: String,
}

/// Compatibility alias for the S3 event notification parser model name.
pub type S3EventNotificationModel = S3EventNotification;

/// Compatibility alias for the S3 event notification record parser model name.
pub type S3EventNotificationRecordModel = S3EventNotificationRecord;

/// Compatibility alias for the S3 record parser model name.
pub type S3RecordModel = S3EventNotificationRecord;

/// Compatibility alias for the S3 Intelligent-Tiering data parser model name.
pub type S3EventRecordIntelligentTieringEventData = S3EventNotificationIntelligentTieringEventData;

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::S3EventNotification;

    #[test]
    fn parses_intelligent_tiering_get_object() {
        let event = serde_json::from_value::<S3EventNotification>(json!({
            "Records": [
                {
                    "eventVersion": "2.3",
                    "eventSource": "aws:s3",
                    "awsRegion": "ap-southeast-2",
                    "eventTime": "2025-09-29T00:47:23.967Z",
                    "eventName": "IntelligentTiering",
                    "userIdentity": {
                        "principalId": "s3.amazonaws.com"
                    },
                    "requestParameters": {
                        "sourceIPAddress": "s3.amazonaws.com"
                    },
                    "responseElements": {
                        "x-amz-request-id": "request-1",
                        "x-amz-id-2": "host-1"
                    },
                    "s3": {
                        "s3SchemaVersion": "1.0",
                        "configurationId": "config-1",
                        "bucket": {
                            "name": "orders",
                            "ownerIdentity": {
                                "principalId": "owner-1"
                            },
                            "arn": "arn:aws:s3:::orders"
                        },
                        "get_object": {
                            "key": "archive/order-1.json",
                            "size": 252_294,
                            "eTag": "etag-1",
                            "versionId": "version-1",
                            "sequencer": "001"
                        }
                    },
                    "intelligentTieringEventData": {
                        "destinationAccessTier": "ARCHIVE_ACCESS"
                    }
                }
            ]
        }))
        .expect("S3 Intelligent-Tiering event should parse");

        let record = &event.records[0];
        assert_eq!(record.event_name, "IntelligentTiering");
        assert_eq!(record.s3.object, None);
        assert_eq!(
            record.s3.object().map(|object| object.key.as_str()),
            Some("archive/order-1.json")
        );
        assert_eq!(
            record
                .intelligent_tiering_event_data
                .as_ref()
                .map(|data| data.destination_access_tier.as_str()),
            Some("ARCHIVE_ACCESS")
        );
    }

    #[test]
    fn serializes_get_object_field_name() {
        let event = serde_json::from_value::<S3EventNotification>(json!({
            "Records": [
                {
                    "eventVersion": "2.3",
                    "eventSource": "aws:s3",
                    "awsRegion": "us-east-1",
                    "eventTime": "2025-09-29T00:47:23.967Z",
                    "eventName": "IntelligentTiering",
                    "userIdentity": {
                        "principalId": "s3.amazonaws.com"
                    },
                    "requestParameters": {
                        "sourceIPAddress": "s3.amazonaws.com"
                    },
                    "responseElements": {
                        "x-amz-request-id": "request-1",
                        "x-amz-id-2": "host-1"
                    },
                    "s3": {
                        "s3SchemaVersion": "1.0",
                        "configurationId": "config-1",
                        "bucket": {
                            "name": "orders",
                            "ownerIdentity": {
                                "principalId": "owner-1"
                            },
                            "arn": "arn:aws:s3:::orders"
                        },
                        "get_object": {
                            "key": "archive/order-1.json",
                            "eTag": "etag-1"
                        }
                    }
                }
            ]
        }))
        .expect("S3 Intelligent-Tiering event should parse");

        let encoded = serde_json::to_value(event).expect("event should serialize");

        assert_eq!(
            encoded["Records"][0]["s3"]["get_object"]["key"],
            "archive/order-1.json"
        );
        assert!(encoded["Records"][0]["s3"].get("object").is_none());
        assert_eq!(encoded["Records"][0]["s3"]["get_object"]["eTag"], "etag-1");
    }
}
