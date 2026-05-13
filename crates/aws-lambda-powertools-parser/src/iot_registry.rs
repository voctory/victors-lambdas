//! AWS `IoT Core` registry event models.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// AWS `IoT Core` registry event type.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub enum IoTCoreRegistryEventType {
    /// Thing created, updated, or deleted event.
    #[serde(rename = "THING_EVENT")]
    Thing,
    /// Thing type created, updated, deprecated, undeprecated, or deleted event.
    #[serde(rename = "THING_TYPE_EVENT")]
    ThingType,
    /// Thing type associated or disassociated with a thing event.
    #[serde(rename = "THING_TYPE_ASSOCIATION_EVENT")]
    ThingTypeAssociation,
    /// Thing group created, updated, or deleted event.
    #[serde(rename = "THING_GROUP_EVENT")]
    ThingGroup,
    /// Thing added to or removed from a thing group event.
    #[serde(rename = "THING_GROUP_MEMBERSHIP_EVENT")]
    ThingGroupMembership,
    /// Child thing group added to or removed from a parent thing group event.
    #[serde(rename = "THING_GROUP_HIERARCHY_EVENT")]
    ThingGroupHierarchy,
}

/// Create, update, or delete operation for AWS `IoT Core` registry events.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IoTCoreRegistryCrudOperation {
    /// Resource created.
    Created,
    /// Resource updated.
    Updated,
    /// Resource deleted.
    Deleted,
}

/// Add or remove operation for AWS `IoT Core` registry membership events.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IoTCoreRegistryMembershipOperation {
    /// Resource added to a registry relationship.
    Added,
    /// Resource removed from a registry relationship.
    Removed,
}

/// AWS `IoT Core` thing created, updated, or deleted event.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IoTCoreThingEvent {
    /// Registry event type.
    pub event_type: IoTCoreRegistryEventType,
    /// Unique event ID.
    pub event_id: String,
    /// Time the registry event occurred.
    #[serde(
        deserialize_with = "unix_timestamp::deserialize",
        serialize_with = "unix_timestamp::serialize"
    )]
    pub timestamp: DateTime<Utc>,
    /// Registry operation that triggered the event.
    pub operation: IoTCoreRegistryCrudOperation,
    /// AWS account ID associated with the event.
    pub account_id: String,
    /// ID of the thing being created, updated, or deleted.
    pub thing_id: String,
    /// Name of the thing being created, updated, or deleted.
    pub thing_name: String,
    /// Version number of the thing.
    pub version_number: u64,
    /// Associated thing type name, when one exists.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thing_type_name: Option<String>,
    /// Thing attributes included in the registry event.
    pub attributes: BTreeMap<String, Value>,
}

/// AWS `IoT Core` thing type created, updated, deprecated, undeprecated, or deleted event.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IoTCoreThingTypeEvent {
    /// Registry event type.
    pub event_type: IoTCoreRegistryEventType,
    /// Unique event ID.
    pub event_id: String,
    /// Time the registry event occurred.
    #[serde(
        deserialize_with = "unix_timestamp::deserialize",
        serialize_with = "unix_timestamp::serialize"
    )]
    pub timestamp: DateTime<Utc>,
    /// Registry operation that triggered the event.
    pub operation: IoTCoreRegistryCrudOperation,
    /// AWS account ID associated with the event.
    pub account_id: String,
    /// ID of the thing type being changed.
    pub thing_type_id: String,
    /// Name of the thing type being changed.
    pub thing_type_name: String,
    /// Whether the thing type is deprecated.
    pub is_deprecated: bool,
    /// Time the thing type was deprecated, when present.
    #[serde(
        default,
        deserialize_with = "unix_timestamp::deserialize_option",
        serialize_with = "unix_timestamp::serialize_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub deprecation_date: Option<DateTime<Utc>>,
    /// Attributes configured as searchable for this thing type.
    pub searchable_attributes: Vec<String>,
    /// Attributes propagated for message enrichment.
    pub propagating_attributes: Vec<IoTCorePropagatingAttribute>,
    /// Thing type description.
    pub description: String,
}

/// Attribute propagation configuration for an AWS `IoT Core` thing type event.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IoTCorePropagatingAttribute {
    /// MQTT user property key populated by AWS `IoT Core`.
    pub user_property_key: String,
    /// Thing attribute to propagate, when configured.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thing_attribute: Option<String>,
    /// Connection attribute to propagate, when configured.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connection_attribute: Option<String>,
}

/// AWS `IoT Core` thing type associated or disassociated with a thing event.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IoTCoreThingTypeAssociationEvent {
    /// Registry event type.
    pub event_type: IoTCoreRegistryEventType,
    /// Unique event ID.
    pub event_id: String,
    /// Time the registry event occurred.
    #[serde(
        deserialize_with = "unix_timestamp::deserialize",
        serialize_with = "unix_timestamp::serialize"
    )]
    pub timestamp: DateTime<Utc>,
    /// Registry membership operation that triggered the event.
    pub operation: IoTCoreRegistryMembershipOperation,
    /// ID of the thing whose type association changed.
    pub thing_id: String,
    /// Name of the thing whose type association changed.
    pub thing_name: String,
    /// Thing type that was associated or disassociated.
    pub thing_type_name: String,
}

/// AWS `IoT Core` thing group created, updated, or deleted event.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IoTCoreThingGroupEvent {
    /// Registry event type.
    pub event_type: IoTCoreRegistryEventType,
    /// Unique event ID.
    pub event_id: String,
    /// Time the registry event occurred.
    #[serde(
        deserialize_with = "unix_timestamp::deserialize",
        serialize_with = "unix_timestamp::serialize"
    )]
    pub timestamp: DateTime<Utc>,
    /// Registry operation that triggered the event.
    pub operation: IoTCoreRegistryCrudOperation,
    /// AWS account ID associated with the event.
    pub account_id: String,
    /// ID of the thing group being created, updated, or deleted.
    pub thing_group_id: String,
    /// Name of the thing group being created, updated, or deleted.
    pub thing_group_name: String,
    /// Version number of the thing group.
    pub version_number: u64,
    /// Parent thing group name, when one exists.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_group_name: Option<String>,
    /// Parent thing group ID, when one exists.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_group_id: Option<String>,
    /// Thing group description.
    pub description: String,
    /// Parent thing groups from the root to this thing group's parent.
    pub root_to_parent_thing_groups: Vec<IoTCoreThingGroupReference>,
    /// Thing group attributes included in the registry event.
    pub attributes: BTreeMap<String, Value>,
    /// Dynamic group mapping ID, when the event is for a dynamic thing group.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dynamic_group_mapping_id: Option<String>,
}

/// Thing group reference in an AWS `IoT Core` registry event.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IoTCoreThingGroupReference {
    /// Thing group ARN.
    pub group_arn: String,
    /// Thing group ID.
    pub group_id: String,
}

/// AWS `IoT Core` thing added to or removed from a thing group event.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IoTCoreThingGroupMembershipEvent {
    /// Registry event type.
    pub event_type: IoTCoreRegistryEventType,
    /// Unique event ID.
    pub event_id: String,
    /// Time the registry event occurred.
    #[serde(
        deserialize_with = "unix_timestamp::deserialize",
        serialize_with = "unix_timestamp::serialize"
    )]
    pub timestamp: DateTime<Utc>,
    /// Registry membership operation that triggered the event.
    pub operation: IoTCoreRegistryMembershipOperation,
    /// AWS account ID associated with the event.
    pub account_id: String,
    /// ARN of the thing group.
    pub group_arn: String,
    /// ID of the thing group.
    pub group_id: String,
    /// ARN of the thing added to or removed from the thing group.
    pub thing_arn: String,
    /// ID of the thing added to or removed from the thing group.
    pub thing_id: String,
    /// ID of the relationship between the thing and the thing group.
    pub membership_id: String,
}

/// Compatibility alias for the thing group membership event model name.
pub type IoTCoreAddOrRemoveFromThingGroupEvent = IoTCoreThingGroupMembershipEvent;

/// AWS `IoT Core` child thing group added to or removed from a parent thing group event.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IoTCoreThingGroupHierarchyEvent {
    /// Registry event type.
    pub event_type: IoTCoreRegistryEventType,
    /// Unique event ID.
    pub event_id: String,
    /// Time the registry event occurred.
    #[serde(
        deserialize_with = "unix_timestamp::deserialize",
        serialize_with = "unix_timestamp::serialize"
    )]
    pub timestamp: DateTime<Utc>,
    /// Registry membership operation that triggered the event.
    pub operation: IoTCoreRegistryMembershipOperation,
    /// AWS account ID associated with the event.
    pub account_id: String,
    /// ID of the parent thing group.
    pub thing_group_id: String,
    /// Name of the parent thing group.
    pub thing_group_name: String,
    /// ID of the child thing group.
    pub child_group_id: String,
    /// Name of the child thing group.
    pub child_group_name: String,
}

/// Compatibility alias for the thing group hierarchy event model name.
pub type IoTCoreAddOrDeleteFromThingGroupEvent = IoTCoreThingGroupHierarchyEvent;

mod unix_timestamp {
    use chrono::{DateTime, TimeZone, Utc};
    use serde::{
        Deserialize, Deserializer, Serializer,
        de::{self, IntoDeserializer, Visitor},
    };
    use serde_json::Value;

    const SECOND_TIMESTAMP_MAX_MAGNITUDE: i64 = 10_000_000_000;

    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(TimestampVisitor)
    }

    pub fn deserialize_option<'de, D>(deserializer: D) -> Result<Option<DateTime<Utc>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Option::<Value>::deserialize(deserializer)?;
        match value {
            Some(Value::Null) | None => Ok(None),
            Some(value) => deserialize(value.into_deserializer())
                .map(Some)
                .map_err(de::Error::custom),
        }
    }

    pub fn serialize<S>(timestamp: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_i64(timestamp.timestamp_millis())
    }

    #[expect(
        clippy::ref_option,
        reason = "serde serialize_with receives field references"
    )]
    pub fn serialize_option<S>(
        timestamp: &Option<DateTime<Utc>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match timestamp {
            Some(timestamp) => serialize(timestamp, serializer),
            None => serializer.serialize_none(),
        }
    }

    struct TimestampVisitor;

    impl Visitor<'_> for TimestampVisitor {
        type Value = DateTime<Utc>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("a UNIX timestamp in seconds or milliseconds")
        }

        fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            timestamp_from_unix(value)
                .ok_or_else(|| E::invalid_value(de::Unexpected::Signed(value), &self))
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            let value = i64::try_from(value)
                .map_err(|_| E::invalid_value(de::Unexpected::Unsigned(value), &self))?;

            self.visit_i64(value)
        }

        fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "validated integer timestamp"
            )]
            #[expect(clippy::cast_precision_loss, reason = "i64 bounds need f64 comparison")]
            if value.is_finite()
                && value.fract() == 0.0
                && value >= i64::MIN as f64
                && value <= i64::MAX as f64
            {
                self.visit_i64(value as i64)
            } else {
                Err(E::invalid_value(de::Unexpected::Float(value), &self))
            }
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            if let Ok(timestamp) = value.parse::<i64>() {
                return self.visit_i64(timestamp);
            }

            DateTime::parse_from_rfc3339(value)
                .map(|timestamp| timestamp.to_utc())
                .map_err(|_| E::invalid_value(de::Unexpected::Str(value), &self))
        }
    }

    fn timestamp_from_unix(timestamp: i64) -> Option<DateTime<Utc>> {
        if (-SECOND_TIMESTAMP_MAX_MAGNITUDE..=SECOND_TIMESTAMP_MAX_MAGNITUDE).contains(&timestamp) {
            Utc.timestamp_opt(timestamp, 0).single()
        } else {
            Utc.timestamp_millis_opt(timestamp).single()
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use serde::de::DeserializeOwned;
    use serde_json::{Value, json};

    use super::{
        IoTCoreRegistryCrudOperation, IoTCoreRegistryEventType, IoTCoreRegistryMembershipOperation,
        IoTCoreThingEvent, IoTCoreThingGroupEvent, IoTCoreThingGroupHierarchyEvent,
        IoTCoreThingGroupMembershipEvent, IoTCoreThingTypeAssociationEvent, IoTCoreThingTypeEvent,
    };

    #[test]
    fn parses_thing_event() {
        let event = parse::<IoTCoreThingEvent>(json!({
            "eventType": "THING_EVENT",
            "eventId": "f5ae9b94-8b8e-4d8e-8c8f-b3266dd89853",
            "timestamp": 1_234_567_890_123_i64,
            "operation": "CREATED",
            "accountId": "123456789012",
            "thingId": "b604f69c-aa9a-4d4a-829e-c480e958a0b5",
            "thingName": "MyThing",
            "versionNumber": 1,
            "thingTypeName": null,
            "attributes": {
                "attribute1": "value1",
                "attribute2": "value2",
                "attribute3": "value3"
            }
        }));

        assert_eq!(event.event_type, IoTCoreRegistryEventType::Thing);
        assert_eq!(event.operation, IoTCoreRegistryCrudOperation::Created);
        assert_eq!(event.timestamp, timestamp_millis(1_234_567_890_123));
        assert_eq!(event.account_id, "123456789012");
        assert_eq!(event.thing_name, "MyThing");
        assert_eq!(event.version_number, 1);
        assert_eq!(event.thing_type_name, None);
        assert_eq!(event.attributes["attribute2"], "value2");
    }

    #[test]
    fn parses_thing_type_event() {
        let event = parse::<IoTCoreThingTypeEvent>(json!({
            "eventType": "THING_TYPE_EVENT",
            "eventId": "8827376c-4b05-49a3-9b3b-733729df7ed5",
            "timestamp": 1_234_567_890_123_i64,
            "operation": "UPDATED",
            "accountId": "123456789012",
            "thingTypeId": "c530ae83-32aa-4592-94d3-da29879d1aac",
            "thingTypeName": "MyThingType",
            "isDeprecated": false,
            "deprecationDate": null,
            "searchableAttributes": ["attribute1", "attribute2", "attribute3"],
            "propagatingAttributes": [
                {"userPropertyKey": "key", "thingAttribute": "model"},
                {"userPropertyKey": "key", "connectionAttribute": "iot:ClientId"}
            ],
            "description": "My thing type"
        }));

        assert_eq!(event.event_type, IoTCoreRegistryEventType::ThingType);
        assert_eq!(event.operation, IoTCoreRegistryCrudOperation::Updated);
        assert_eq!(event.timestamp, timestamp_millis(1_234_567_890_123));
        assert_eq!(event.thing_type_name, "MyThingType");
        assert_eq!(event.deprecation_date, None);
        assert_eq!(event.searchable_attributes.len(), 3);
        assert_eq!(
            event.propagating_attributes[0].thing_attribute.as_deref(),
            Some("model")
        );
        assert_eq!(
            event.propagating_attributes[1]
                .connection_attribute
                .as_deref(),
            Some("iot:ClientId")
        );
    }

    #[test]
    fn parses_deprecation_date_from_unix_or_rfc3339_timestamp() {
        let unix_event = parse::<IoTCoreThingTypeEvent>(thing_type_event_with_deprecation_date(
            &json!(1_234_567_890),
        ));
        let rfc3339_event = parse::<IoTCoreThingTypeEvent>(thing_type_event_with_deprecation_date(
            &json!("2009-02-13T23:31:30Z"),
        ));

        let expected = timestamp_seconds(1_234_567_890);
        assert_eq!(unix_event.deprecation_date, Some(expected));
        assert_eq!(rfc3339_event.deprecation_date, Some(expected));
    }

    #[test]
    fn parses_thing_type_association_event() {
        let event = parse::<IoTCoreThingTypeAssociationEvent>(json!({
            "eventId": "87f8e095-531c-47b3-aab5-5171364d138d",
            "eventType": "THING_TYPE_ASSOCIATION_EVENT",
            "operation": "ADDED",
            "thingId": "b604f69c-aa9a-4d4a-829e-c480e958a0b5",
            "thingName": "myThing",
            "thingTypeName": "MyThingType",
            "timestamp": 1_234_567_890_123_i64
        }));

        assert_eq!(
            event.event_type,
            IoTCoreRegistryEventType::ThingTypeAssociation
        );
        assert_eq!(event.operation, IoTCoreRegistryMembershipOperation::Added);
        assert_eq!(event.thing_name, "myThing");
        assert_eq!(event.thing_type_name, "MyThingType");
    }

    #[test]
    fn parses_thing_group_event() {
        let event = parse::<IoTCoreThingGroupEvent>(json!({
            "eventType": "THING_GROUP_EVENT",
            "eventId": "8b9ea8626aeaa1e42100f3f32b975899",
            "timestamp": 1_603_995_417_409_i64,
            "operation": "UPDATED",
            "accountId": "571EXAMPLE833",
            "thingGroupId": "8757eec8-bb37-4cca-a6fa-403b003d139f",
            "thingGroupName": "Tg_level5",
            "versionNumber": 3,
            "parentGroupName": "Tg_level4",
            "parentGroupId": "5fce366a-7875-4c0e-870b-79d8d1dce119",
            "description": "New description for Tg_level5",
            "rootToParentThingGroups": [
                {
                    "groupArn": "arn:aws:iot:us-west-2:571EXAMPLE833:thinggroup/TgTopLevel",
                    "groupId": "36aa0482-f80d-4e13-9bff-1c0a75c055f6"
                }
            ],
            "attributes": {
                "attribute1": "value1",
                "attribute2": "value2",
                "attribute3": "value3"
            },
            "dynamicGroupMappingId": null
        }));

        assert_eq!(event.event_type, IoTCoreRegistryEventType::ThingGroup);
        assert_eq!(event.operation, IoTCoreRegistryCrudOperation::Updated);
        assert_eq!(event.timestamp, timestamp_millis(1_603_995_417_409));
        assert_eq!(event.thing_group_name, "Tg_level5");
        assert_eq!(event.parent_group_name.as_deref(), Some("Tg_level4"));
        assert_eq!(event.root_to_parent_thing_groups.len(), 1);
        assert_eq!(
            event.root_to_parent_thing_groups[0].group_arn,
            "arn:aws:iot:us-west-2:571EXAMPLE833:thinggroup/TgTopLevel"
        );
        assert_eq!(event.dynamic_group_mapping_id, None);
    }

    #[test]
    fn parses_thing_group_membership_event() {
        let event = parse::<IoTCoreThingGroupMembershipEvent>(json!({
            "eventType": "THING_GROUP_MEMBERSHIP_EVENT",
            "eventId": "d684bd5f-6f6e-48e1-950c-766ac7f02fd1",
            "timestamp": 1_234_567_890_123_i64,
            "operation": "REMOVED",
            "accountId": "123456789012",
            "groupArn": "arn:aws:iot:ap-northeast-2:123456789012:thinggroup/MyChildThingGroup",
            "groupId": "06838589-373f-4312-b1f2-53f2192291c4",
            "thingArn": "arn:aws:iot:ap-northeast-2:123456789012:thing/MyThing",
            "thingId": "b604f69c-aa9a-4d4a-829e-c480e958a0b5",
            "membershipId": "8505ebf8-4d32-4286-80e9-c23a4a16bbd8"
        }));

        assert_eq!(
            event.event_type,
            IoTCoreRegistryEventType::ThingGroupMembership
        );
        assert_eq!(event.operation, IoTCoreRegistryMembershipOperation::Removed);
        assert_eq!(
            event.group_arn,
            "arn:aws:iot:ap-northeast-2:123456789012:thinggroup/MyChildThingGroup"
        );
        assert_eq!(event.membership_id, "8505ebf8-4d32-4286-80e9-c23a4a16bbd8");
    }

    #[test]
    fn parses_thing_group_hierarchy_event() {
        let event = parse::<IoTCoreThingGroupHierarchyEvent>(json!({
            "eventType": "THING_GROUP_HIERARCHY_EVENT",
            "eventId": "264192c7-b573-46ef-ab7b-489fcd47da41",
            "timestamp": 1_234_567_890_123_i64,
            "operation": "ADDED",
            "accountId": "123456789012",
            "thingGroupId": "8f82a106-6b1d-4331-8984-a84db5f6f8cb",
            "thingGroupName": "MyRootThingGroup",
            "childGroupId": "06838589-373f-4312-b1f2-53f2192291c4",
            "childGroupName": "MyChildThingGroup"
        }));

        assert_eq!(
            event.event_type,
            IoTCoreRegistryEventType::ThingGroupHierarchy
        );
        assert_eq!(event.operation, IoTCoreRegistryMembershipOperation::Added);
        assert_eq!(event.thing_group_name, "MyRootThingGroup");
        assert_eq!(event.child_group_name, "MyChildThingGroup");
    }

    #[test]
    fn serializes_timestamp_as_unix_milliseconds() {
        let event = parse::<IoTCoreThingTypeAssociationEvent>(json!({
            "eventId": "87f8e095-531c-47b3-aab5-5171364d138d",
            "eventType": "THING_TYPE_ASSOCIATION_EVENT",
            "operation": "ADDED",
            "thingId": "b604f69c-aa9a-4d4a-829e-c480e958a0b5",
            "thingName": "myThing",
            "thingTypeName": "MyThingType",
            "timestamp": "2009-02-13T23:31:30Z"
        }));

        let encoded = serde_json::to_value(event).expect("event should serialize");
        assert_eq!(encoded["timestamp"], 1_234_567_890_000i64);
    }

    fn parse<T>(value: Value) -> T
    where
        T: DeserializeOwned,
    {
        serde_json::from_value(value).expect("registry event should parse")
    }

    fn timestamp_millis(timestamp: i64) -> chrono::DateTime<chrono::Utc> {
        chrono::Utc
            .timestamp_millis_opt(timestamp)
            .single()
            .expect("timestamp should be valid")
    }

    fn timestamp_seconds(timestamp: i64) -> chrono::DateTime<chrono::Utc> {
        chrono::Utc
            .timestamp_opt(timestamp, 0)
            .single()
            .expect("timestamp should be valid")
    }

    fn thing_type_event_with_deprecation_date(deprecation_date: &Value) -> Value {
        json!({
            "eventType": "THING_TYPE_EVENT",
            "eventId": "8827376c-4b05-49a3-9b3b-733729df7ed5",
            "timestamp": 1_234_567_890_123_i64,
            "operation": "UPDATED",
            "accountId": "123456789012",
            "thingTypeId": "c530ae83-32aa-4592-94d3-da29879d1aac",
            "thingTypeName": "MyThingType",
            "isDeprecated": true,
            "deprecationDate": deprecation_date,
            "searchableAttributes": ["attribute1", "attribute2", "attribute3"],
            "propagatingAttributes": [
                {"userPropertyKey": "key", "thingAttribute": "model"}
            ],
            "description": "My thing type"
        })
    }
}
