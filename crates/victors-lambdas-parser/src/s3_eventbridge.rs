//! Amazon S3 `EventBridge` notification models.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Amazon S3 bucket metadata in an `EventBridge` notification detail.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct S3EventBridgeBucket {
    /// Bucket name.
    pub name: String,
}

/// Amazon S3 object metadata in an `EventBridge` notification detail.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct S3EventBridgeObject {
    /// Object key.
    pub key: String,
    /// Object size in bytes, when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    /// Object entity tag, when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub etag: Option<String>,
    /// Object version ID, when present.
    #[serde(
        default,
        rename = "version-id",
        skip_serializing_if = "Option::is_none"
    )]
    pub version_id: Option<String>,
    /// Object sequencer, when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sequencer: Option<String>,
}

/// Amazon S3 `EventBridge` notification detail.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct S3EventBridgeDetail {
    /// Detail schema version.
    pub version: String,
    /// Bucket associated with the S3 event.
    pub bucket: S3EventBridgeBucket,
    /// Object associated with the S3 event.
    pub object: S3EventBridgeObject,
    /// S3 request ID.
    #[serde(rename = "request-id")]
    pub request_id: String,
    /// AWS account ID or service principal of the requester.
    pub requester: String,
    /// Source IP address of the S3 request, when present.
    #[serde(
        default,
        rename = "source-ip-address",
        skip_serializing_if = "Option::is_none"
    )]
    pub source_ip_address: Option<String>,
    /// Reason for the S3 event, when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Deletion type for object-deleted events, when present.
    #[serde(
        default,
        rename = "deletion-type",
        skip_serializing_if = "Option::is_none"
    )]
    pub deletion_type: Option<String>,
    /// Restore expiry time for object-restore-completed events, when present.
    #[serde(
        default,
        rename = "restore-expiry-time",
        skip_serializing_if = "Option::is_none"
    )]
    pub restore_expiry_time: Option<String>,
    /// Source storage class for object-restore events, when present.
    #[serde(
        default,
        rename = "source-storage-class",
        skip_serializing_if = "Option::is_none"
    )]
    pub source_storage_class: Option<String>,
    /// Destination storage class for storage-class-changed events, when present.
    #[serde(
        default,
        rename = "destination-storage-class",
        skip_serializing_if = "Option::is_none"
    )]
    pub destination_storage_class: Option<String>,
    /// Destination access tier for access-tier-changed events, when present.
    #[serde(
        default,
        rename = "destination-access-tier",
        skip_serializing_if = "Option::is_none"
    )]
    pub destination_access_tier: Option<String>,
}

/// Amazon S3 `EventBridge` notification.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct S3EventBridgeEvent {
    /// `EventBridge` event version.
    pub version: String,
    /// Event ID.
    pub id: String,
    /// S3 `EventBridge` detail type.
    #[serde(rename = "detail-type")]
    pub detail_type: String,
    /// Event source, normally `aws.s3`.
    pub source: String,
    /// AWS account ID that owns the event.
    pub account: String,
    /// Time `EventBridge` received the event.
    pub time: DateTime<Utc>,
    /// AWS Region where the event occurred.
    pub region: String,
    /// Resource ARNs associated with the event.
    pub resources: Vec<String>,
    /// S3-specific event detail.
    pub detail: S3EventBridgeDetail,
}

/// Compatibility alias for the S3 `EventBridge` detail parser model name.
pub type S3EventNotificationEventBridgeDetailModel = S3EventBridgeDetail;

/// Compatibility alias for the S3 `EventBridge` parser model name.
pub type S3EventNotificationEventBridgeModel = S3EventBridgeEvent;

#[cfg(test)]
mod tests {
    use chrono::DateTime;
    use serde_json::json;

    use super::{S3EventBridgeDetail, S3EventBridgeEvent};

    #[test]
    fn parses_s3_eventbridge_object_created_event() {
        let event = serde_json::from_value::<S3EventBridgeEvent>(json!({
            "version": "0",
            "id": "f5f1e65c-dc3a-93ca-6c1e-b1647eac7963",
            "detail-type": "Object Created",
            "source": "aws.s3",
            "account": "123456789012",
            "time": "2023-03-08T17:50:14Z",
            "region": "eu-west-1",
            "resources": [
                "arn:aws:s3:::example-bucket"
            ],
            "detail": {
                "version": "0",
                "bucket": {
                    "name": "example-bucket"
                },
                "object": {
                    "key": "IMG_m7fzo3.jpg",
                    "size": 184_662,
                    "etag": "4e68adba0abe2dc8653dc3354e14c01d",
                    "sequencer": "006408CAD69598B05E"
                },
                "request-id": "57H08PA84AB1JZW0",
                "requester": "123456789012",
                "source-ip-address": "34.252.34.74",
                "reason": "PutObject"
            }
        }))
        .expect("S3 EventBridge event should parse");

        assert_eq!(event.detail_type, "Object Created");
        assert_eq!(event.source, "aws.s3");
        assert_eq!(
            event.time,
            DateTime::parse_from_rfc3339("2023-03-08T17:50:14Z")
                .expect("timestamp should parse")
                .to_utc()
        );
        assert_eq!(event.detail.bucket.name, "example-bucket");
        assert_eq!(event.detail.object.key, "IMG_m7fzo3.jpg");
        assert_eq!(event.detail.object.size, Some(184_662));
        assert_eq!(event.detail.reason.as_deref(), Some("PutObject"));
    }

    #[test]
    fn parses_s3_eventbridge_object_deleted_detail() {
        let detail = serde_json::from_value::<S3EventBridgeDetail>(json!({
            "version": "0",
            "bucket": {
                "name": "example-bucket"
            },
            "object": {
                "key": "expired-object.txt",
                "version-id": "096fKKXTRTtl3on89fVO.nfljtsv6qko",
                "sequencer": "006408CAD69598B05E"
            },
            "request-id": "57H08PA84AB1JZW0",
            "requester": "s3.amazonaws.com",
            "reason": "Lifecycle Expiration",
            "deletion-type": "Permanently Deleted"
        }))
        .expect("S3 EventBridge detail should parse");

        assert_eq!(detail.object.key, "expired-object.txt");
        assert_eq!(detail.object.size, None);
        assert_eq!(detail.object.etag, None);
        assert_eq!(
            detail.object.version_id.as_deref(),
            Some("096fKKXTRTtl3on89fVO.nfljtsv6qko")
        );
        assert_eq!(detail.deletion_type.as_deref(), Some("Permanently Deleted"));
    }

    #[test]
    fn serializes_hyphenated_s3_eventbridge_fields() {
        let detail = serde_json::from_value::<S3EventBridgeDetail>(json!({
            "version": "0",
            "bucket": {
                "name": "example-bucket"
            },
            "object": {
                "key": "archive.zip",
                "version-id": "version-1"
            },
            "request-id": "57H08PA84AB1JZW0",
            "requester": "123456789012",
            "source-storage-class": "GLACIER",
            "destination-storage-class": "STANDARD",
            "destination-access-tier": "ARCHIVE_ACCESS",
            "restore-expiry-time": "2021-11-13T00:00:00Z"
        }))
        .expect("S3 EventBridge detail should parse");

        let encoded = serde_json::to_value(detail).expect("detail should serialize");

        assert_eq!(encoded["object"]["version-id"], "version-1");
        assert_eq!(encoded["request-id"], "57H08PA84AB1JZW0");
        assert_eq!(encoded["source-storage-class"], "GLACIER");
        assert_eq!(encoded["destination-storage-class"], "STANDARD");
        assert_eq!(encoded["destination-access-tier"], "ARCHIVE_ACCESS");
        assert_eq!(encoded["restore-expiry-time"], "2021-11-13T00:00:00Z");
        assert_eq!(encoded["object"].get("size"), None);
    }
}
