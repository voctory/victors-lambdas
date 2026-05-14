//! Kafka consumer record materialization.

use std::{collections::HashMap, marker::PhantomData, slice};

use aws_lambda_events::event::kafka::{KafkaEvent, KafkaRecord as LambdaKafkaRecord};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use chrono::{DateTime, Utc};
use serde::de::DeserializeOwned;

use crate::{
    KafkaConsumerConfig, KafkaConsumerError, KafkaConsumerResult, KafkaField, KafkaFieldDecoder,
    KafkaSchemaConfig, KafkaSchemaMetadata, config::KafkaFieldDeserializer,
};

/// A flattened Kafka record with decoded key, value, and headers.
#[derive(Clone, Debug, PartialEq)]
pub struct ConsumerRecord<K, V> {
    /// Key from the source `KafkaEvent.records` map, typically topic-partition.
    pub source_key: String,
    /// Kafka topic name.
    pub topic: Option<String>,
    /// Kafka partition number.
    pub partition: i64,
    /// Kafka record offset.
    pub offset: i64,
    /// Kafka record timestamp.
    pub timestamp: DateTime<Utc>,
    /// Kafka timestamp type.
    pub timestamp_type: Option<String>,
    /// Decoded key, if present.
    pub key: Option<K>,
    /// Decoded value, if present.
    pub value: Option<V>,
    /// Original base64-encoded key, if present.
    pub original_key: Option<String>,
    /// Original base64-encoded value, if present.
    pub original_value: Option<String>,
    /// Schema metadata for the key supplied by Lambda Event Source Mapping.
    pub key_schema_metadata: Option<KafkaSchemaMetadata>,
    /// Schema metadata for the value supplied by Lambda Event Source Mapping.
    pub value_schema_metadata: Option<KafkaSchemaMetadata>,
    /// Headers decoded from byte arrays into `UTF-8` strings.
    pub headers: Vec<HashMap<String, String>>,
    /// Original header byte arrays from the Lambda event.
    pub original_headers: Vec<HashMap<String, Vec<i8>>>,
}

/// Flattened Kafka records plus event-level metadata.
#[derive(Clone, Debug, PartialEq)]
pub struct ConsumerRecords<K, V> {
    /// Lambda event source.
    pub event_source: Option<String>,
    /// Lambda event source ARN.
    pub event_source_arn: Option<String>,
    /// Kafka bootstrap servers reported by the Lambda event.
    pub bootstrap_servers: Option<String>,
    /// Flattened records from every topic-partition group.
    pub records: Vec<ConsumerRecord<K, V>>,
}

impl<K, V> ConsumerRecords<K, V> {
    /// Returns the number of flattened Kafka records.
    #[must_use]
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Returns true when the event contained no Kafka records.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Iterates over flattened Kafka records.
    pub fn iter(&self) -> slice::Iter<'_, ConsumerRecord<K, V>> {
        self.records.iter()
    }
}

impl<K, V> IntoIterator for ConsumerRecords<K, V> {
    type Item = ConsumerRecord<K, V>;
    type IntoIter = std::vec::IntoIter<ConsumerRecord<K, V>>;

    fn into_iter(self) -> Self::IntoIter {
        self.records.into_iter()
    }
}

impl<'records, K, V> IntoIterator for &'records ConsumerRecords<K, V> {
    type Item = &'records ConsumerRecord<K, V>;
    type IntoIter = slice::Iter<'records, ConsumerRecord<K, V>>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// Kafka consumer helper that decodes Lambda Kafka event records.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KafkaConsumer<K, V> {
    config: KafkaConsumerConfig,
    marker: PhantomData<fn() -> (K, V)>,
}

/// Schema-aware Kafka consumer helper that decodes Lambda Kafka event records.
#[derive(Clone, Debug)]
pub struct KafkaSchemaConsumer<K, V> {
    config: KafkaSchemaConfig<K, V>,
}

impl<K, V> KafkaSchemaConsumer<K, V> {
    /// Creates a schema-aware Kafka consumer helper.
    #[must_use]
    pub const fn new(config: KafkaSchemaConfig<K, V>) -> Self {
        Self { config }
    }

    /// Returns the configured Kafka schema decoders.
    #[must_use]
    pub const fn config(&self) -> &KafkaSchemaConfig<K, V> {
        &self.config
    }

    /// Decodes and flattens records from a Lambda Kafka event with schema-aware decoders.
    ///
    /// # Errors
    ///
    /// Returns an error when a configured key, value, or header cannot be decoded.
    pub fn records(&self, event: KafkaEvent) -> KafkaConsumerResult<ConsumerRecords<K, V>> {
        schema_consumer_records(event, &self.config)
    }
}

impl<K, V> Default for KafkaConsumer<K, V> {
    fn default() -> Self {
        Self::new(KafkaConsumerConfig::new())
    }
}

impl<K, V> KafkaConsumer<K, V> {
    /// Creates a Kafka consumer helper.
    #[must_use]
    pub const fn new(config: KafkaConsumerConfig) -> Self {
        Self {
            config,
            marker: PhantomData,
        }
    }

    /// Returns the configured Kafka key and value deserializers.
    #[must_use]
    pub const fn config(&self) -> KafkaConsumerConfig {
        self.config
    }
}

impl<K, V> KafkaConsumer<K, V>
where
    K: DeserializeOwned,
    V: DeserializeOwned,
{
    /// Decodes and flattens records from a Lambda Kafka event.
    ///
    /// # Errors
    ///
    /// Returns an error when a configured key, value, or header cannot be decoded.
    pub fn records(&self, event: KafkaEvent) -> KafkaConsumerResult<ConsumerRecords<K, V>> {
        consumer_records(event, self.config)
    }
}

/// Decodes base64 bytes as a `UTF-8` string.
///
/// # Errors
///
/// Returns an error when the input is not valid base64 or the decoded bytes are not valid `UTF-8`.
pub fn decode_base64_string(encoded: &str) -> KafkaConsumerResult<String> {
    decode_base64_string_field("value", encoded)
}

/// Decodes base64 bytes as `JSON` and deserializes them into `T`.
///
/// # Errors
///
/// Returns an error when the input is not valid base64 or the decoded bytes cannot be deserialized as `JSON`.
pub fn decode_base64_json<T>(encoded: &str) -> KafkaConsumerResult<T>
where
    T: DeserializeOwned,
{
    decode_base64_json_field("value", encoded)
}

/// Decodes and flattens records from a Lambda Kafka event.
///
/// Records are flattened in sorted source-key order and keep their order within each source-key group.
///
/// # Errors
///
/// Returns an error when a configured key, value, or header cannot be decoded.
pub fn consumer_records<K, V>(
    event: KafkaEvent,
    config: KafkaConsumerConfig,
) -> KafkaConsumerResult<ConsumerRecords<K, V>>
where
    K: DeserializeOwned,
    V: DeserializeOwned,
{
    flatten_records(event, |source_key, record| {
        decode_record(source_key, record, config)
    })
}

/// Decodes and flattens records from a Lambda Kafka event with schema-aware decoders.
///
/// Records are flattened in sorted source-key order and keep their order within each source-key group.
///
/// # Errors
///
/// Returns an error when a configured key, value, or header cannot be decoded.
pub fn schema_consumer_records<K, V>(
    event: KafkaEvent,
    config: &KafkaSchemaConfig<K, V>,
) -> KafkaConsumerResult<ConsumerRecords<K, V>> {
    flatten_records(event, |source_key, record| {
        decode_record_with_schema(source_key, record, config)
    })
}

fn flatten_records<K, V>(
    event: KafkaEvent,
    mut decode: impl FnMut(String, LambdaKafkaRecord) -> KafkaConsumerResult<ConsumerRecord<K, V>>,
) -> KafkaConsumerResult<ConsumerRecords<K, V>> {
    let KafkaEvent {
        event_source,
        event_source_arn,
        records,
        bootstrap_servers,
        ..
    } = event;

    let mut grouped_records: Vec<_> = records.into_iter().collect();
    grouped_records.sort_by(|(left, _), (right, _)| left.cmp(right));

    let mut flattened = Vec::new();
    for (source_key, records) in grouped_records {
        flattened.reserve(records.len());
        for record in records {
            flattened.push(decode(source_key.clone(), record)?);
        }
    }

    Ok(ConsumerRecords {
        event_source,
        event_source_arn,
        bootstrap_servers,
        records: flattened,
    })
}

fn decode_record<K, V>(
    source_key: String,
    record: LambdaKafkaRecord,
    config: KafkaConsumerConfig,
) -> KafkaConsumerResult<ConsumerRecord<K, V>>
where
    K: DeserializeOwned,
    V: DeserializeOwned,
{
    let key_schema_metadata = schema_metadata(&record, "keySchemaMetadata");
    let value_schema_metadata = schema_metadata(&record, "valueSchemaMetadata");
    let key = deserialize_optional_field(record.key.as_deref(), config.key_deserializer(), "key")?;
    let value = deserialize_optional_field(
        record.value.as_deref(),
        config.value_deserializer(),
        "value",
    )?;
    let headers = decode_headers(&record.headers)?;

    Ok(ConsumerRecord {
        source_key,
        topic: record.topic,
        partition: record.partition,
        offset: record.offset,
        timestamp: record.timestamp.0,
        timestamp_type: record.timestamp_type,
        key,
        value,
        original_key: record.key,
        original_value: record.value,
        key_schema_metadata,
        value_schema_metadata,
        headers,
        original_headers: record.headers,
    })
}

fn decode_record_with_schema<K, V>(
    source_key: String,
    record: LambdaKafkaRecord,
    config: &KafkaSchemaConfig<K, V>,
) -> KafkaConsumerResult<ConsumerRecord<K, V>> {
    let key_schema_metadata = schema_metadata(&record, "keySchemaMetadata");
    let value_schema_metadata = schema_metadata(&record, "valueSchemaMetadata");
    let key = decode_optional_with_decoder(
        record.key.as_deref(),
        config.key_decoder(),
        key_schema_metadata.as_ref(),
        KafkaField::Key,
    )?;
    let value = decode_optional_with_decoder(
        record.value.as_deref(),
        config.value_decoder(),
        value_schema_metadata.as_ref(),
        KafkaField::Value,
    )?;
    let headers = decode_headers(&record.headers)?;

    Ok(ConsumerRecord {
        source_key,
        topic: record.topic,
        partition: record.partition,
        offset: record.offset,
        timestamp: record.timestamp.0,
        timestamp_type: record.timestamp_type,
        key,
        value,
        original_key: record.key,
        original_value: record.value,
        key_schema_metadata,
        value_schema_metadata,
        headers,
        original_headers: record.headers,
    })
}

fn schema_metadata(record: &LambdaKafkaRecord, field: &str) -> Option<KafkaSchemaMetadata> {
    record
        .other
        .get(field)
        .and_then(KafkaSchemaMetadata::from_value)
}

fn decode_optional_with_decoder<T>(
    encoded: Option<&str>,
    decoder: &dyn KafkaFieldDecoder<T>,
    metadata: Option<&KafkaSchemaMetadata>,
    field: KafkaField,
) -> KafkaConsumerResult<Option<T>> {
    match encoded {
        Some(value) if !value.is_empty() => decoder.decode(value, metadata, field).map(Some),
        _ => Ok(None),
    }
}

fn deserialize_optional_field<T>(
    encoded: Option<&str>,
    deserializer: KafkaFieldDeserializer,
    field: &'static str,
) -> KafkaConsumerResult<Option<T>>
where
    T: DeserializeOwned,
{
    match encoded {
        Some(value) if !value.is_empty() => deserialize_field(value, deserializer, field).map(Some),
        _ => Ok(None),
    }
}

fn deserialize_field<T>(
    encoded: &str,
    deserializer: KafkaFieldDeserializer,
    field: &'static str,
) -> KafkaConsumerResult<T>
where
    T: DeserializeOwned,
{
    match deserializer {
        KafkaFieldDeserializer::Primitive => {
            let decoded = decode_base64_string_field(field, encoded)?;
            serde_json::from_value(serde_json::Value::String(decoded))
                .map_err(|error| KafkaConsumerError::json(field, error))
        }
        KafkaFieldDeserializer::Json => decode_base64_json_field(field, encoded),
    }
}

pub(crate) fn decode_base64_string_field(
    field: &'static str,
    encoded: &str,
) -> KafkaConsumerResult<String> {
    let bytes = decode_base64_field(field, encoded)?;
    String::from_utf8(bytes).map_err(|error| KafkaConsumerError::utf8(field, error))
}

pub(crate) fn decode_base64_json_field<T>(
    field: &'static str,
    encoded: &str,
) -> KafkaConsumerResult<T>
where
    T: DeserializeOwned,
{
    let bytes = decode_base64_field(field, encoded)?;
    serde_json::from_slice(&bytes).map_err(|error| KafkaConsumerError::json(field, error))
}

pub(crate) fn decode_base64_field(
    field: &'static str,
    encoded: &str,
) -> KafkaConsumerResult<Vec<u8>> {
    STANDARD
        .decode(encoded)
        .map_err(|error| KafkaConsumerError::base64(field, error))
}

fn decode_headers(
    headers: &[HashMap<String, Vec<i8>>],
) -> KafkaConsumerResult<Vec<HashMap<String, String>>> {
    let mut decoded_headers = Vec::with_capacity(headers.len());

    for header in headers {
        let mut decoded_header = HashMap::with_capacity(header.len());

        for (name, value) in header {
            let bytes = value.iter().map(|byte| byte.to_ne_bytes()[0]).collect();
            let decoded = String::from_utf8(bytes)
                .map_err(|error| KafkaConsumerError::header(name, error))?;
            decoded_header.insert(name.clone(), decoded);
        }

        decoded_headers.push(decoded_header);
    }

    Ok(decoded_headers)
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "protobuf")]
    use prost::Message;
    use serde::Deserialize;
    use serde_json::json;

    #[cfg(feature = "avro")]
    use crate::AvroKafkaFieldDecoder;
    #[cfg(feature = "protobuf")]
    use crate::ProtobufKafkaFieldDecoder;
    use crate::{JsonKafkaFieldDecoder, PrimitiveKafkaFieldDecoder};

    use super::*;

    #[derive(Debug, Deserialize, PartialEq)]
    struct Order {
        order_id: String,
        quantity: u32,
    }

    #[cfg(feature = "protobuf")]
    #[derive(Clone, PartialEq, prost::Message)]
    struct ProtoOrder {
        #[prost(string, tag = "1")]
        order_id: String,
        #[prost(uint32, tag = "2")]
        quantity: u32,
    }

    #[test]
    fn decodes_primitive_records() {
        let key = STANDARD.encode("customer-1");
        let value = STANDARD.encode("created");
        let event = kafka_event(&json!({
            "orders-0": [{
                "topic": "orders",
                "partition": 0,
                "offset": 15,
                "timestamp": 1_690_900_000_000_i64,
                "timestampType": "CREATE_TIME",
                "key": key,
                "value": value,
                "headers": [{"traceparent": [116, 114, 97, 99, 101]}]
            }]
        }));

        let records = KafkaConsumer::<String, String>::default()
            .records(event)
            .expect("record should decode");

        assert_eq!(records.len(), 1);

        let record = records.records.first().expect("record should exist");
        assert_eq!(record.source_key, "orders-0");
        assert_eq!(record.topic.as_deref(), Some("orders"));
        assert_eq!(record.key.as_deref(), Some("customer-1"));
        assert_eq!(record.value.as_deref(), Some("created"));
        assert_eq!(
            record.headers[0].get("traceparent").map(String::as_str),
            Some("trace")
        );
        assert_eq!(
            record.original_headers[0].get("traceparent"),
            Some(&vec![116, 114, 97, 99, 101])
        );
    }

    #[test]
    fn decodes_json_values() {
        let key = STANDARD.encode("customer-1");
        let value = STANDARD.encode(r#"{"order_id":"order-1","quantity":2}"#);
        let event = kafka_event(&json!({
            "orders-0": [{
                "topic": "orders",
                "partition": 0,
                "offset": 16,
                "timestamp": 1_690_900_001_000_i64,
                "timestampType": "CREATE_TIME",
                "key": key,
                "value": value,
                "headers": []
            }]
        }));

        let records = KafkaConsumer::<String, Order>::new(KafkaConsumerConfig::json_values())
            .records(event)
            .expect("record should decode");
        let record = records.records.first().expect("record should exist");

        assert_eq!(record.key.as_deref(), Some("customer-1"));
        assert_eq!(
            record.value.as_ref(),
            Some(&Order {
                order_id: "order-1".to_string(),
                quantity: 2,
            })
        );
        assert_eq!(record.original_value.as_deref(), Some(value.as_str()));
    }

    #[test]
    fn returns_base64_errors() {
        let error = decode_base64_string("not-base64").expect_err("base64 should fail");

        assert_eq!(error.kind(), crate::KafkaConsumerErrorKind::Base64);
    }

    #[test]
    fn returns_json_errors() {
        let encoded = STANDARD.encode("not-json");
        let error = decode_base64_json::<Order>(&encoded).expect_err("JSON should fail");

        assert_eq!(error.kind(), crate::KafkaConsumerErrorKind::Json);
    }

    #[test]
    fn preserves_empty_optional_fields() {
        let event = kafka_event(&json!({
            "orders-0": [{
                "topic": "orders",
                "partition": 0,
                "offset": 17,
                "timestamp": 1_690_900_002_000_i64,
                "timestampType": "CREATE_TIME",
                "key": "",
                "value": null,
                "headers": []
            }]
        }));

        let records = KafkaConsumer::<String, String>::default()
            .records(event)
            .expect("record should decode");
        let record = records.records.first().expect("record should exist");

        assert_eq!(record.key, None);
        assert_eq!(record.value, None);
        assert_eq!(record.original_key.as_deref(), Some(""));
        assert_eq!(record.original_value, None);
    }

    #[test]
    fn schema_consumer_decodes_json_values_and_captures_metadata() {
        let key = STANDARD.encode("customer-1");
        let value = STANDARD.encode(r#"{"order_id":"order-1","quantity":2}"#);
        let event = kafka_event(&json!({
            "orders-0": [{
                "topic": "orders",
                "partition": 0,
                "offset": 18,
                "timestamp": 1_690_900_003_000_i64,
                "timestampType": "CREATE_TIME",
                "key": key,
                "value": value,
                "valueSchemaMetadata": {
                    "dataFormat": "JSON",
                    "schemaId": "json-value-schema"
                },
                "headers": []
            }]
        }));
        let config =
            KafkaSchemaConfig::<String, Order>::new().with_value_decoder(JsonKafkaFieldDecoder);

        let records = KafkaSchemaConsumer::new(config)
            .records(event)
            .expect("record should decode");
        let record = records.records.first().expect("record should exist");

        assert_eq!(record.key.as_deref(), Some("customer-1"));
        assert_eq!(
            record.value.as_ref(),
            Some(&Order {
                order_id: "order-1".to_string(),
                quantity: 2,
            })
        );
        assert_eq!(
            record
                .value_schema_metadata
                .as_ref()
                .and_then(KafkaSchemaMetadata::data_format),
            Some("JSON")
        );
        assert_eq!(
            record
                .value_schema_metadata
                .as_ref()
                .and_then(KafkaSchemaMetadata::schema_id),
            Some("json-value-schema")
        );
    }

    #[test]
    fn schema_consumer_rejects_schema_format_mismatches() {
        let value = STANDARD.encode(r#"{"order_id":"order-1","quantity":2}"#);
        let event = kafka_event(&json!({
            "orders-0": [{
                "topic": "orders",
                "partition": 0,
                "offset": 19,
                "timestamp": 1_690_900_004_000_i64,
                "timestampType": "CREATE_TIME",
                "key": null,
                "value": value,
                "valueSchemaMetadata": {
                    "dataFormat": "AVRO",
                    "schemaId": "avro-value-schema"
                },
                "headers": []
            }]
        }));
        let config =
            KafkaSchemaConfig::<String, Order>::new().with_value_decoder(JsonKafkaFieldDecoder);

        let error = KafkaSchemaConsumer::new(config)
            .records(event)
            .expect_err("metadata mismatch should fail");

        assert_eq!(error.kind(), crate::KafkaConsumerErrorKind::Schema);
    }

    #[cfg(feature = "avro")]
    #[test]
    fn schema_consumer_decodes_avro_values() {
        let schema = r#"{
            "type": "record",
            "name": "Order",
            "fields": [
                {"name": "order_id", "type": "string"},
                {"name": "quantity", "type": "int"}
            ]
        }"#;
        let parsed_schema = apache_avro::Schema::parse_str(schema).expect("schema should parse");
        let value = apache_avro::types::Value::Record(vec![
            (
                "order_id".to_string(),
                apache_avro::types::Value::String("order-1".to_string()),
            ),
            ("quantity".to_string(), apache_avro::types::Value::Int(2)),
        ]);
        let encoded =
            STANDARD.encode(apache_avro::to_avro_datum(&parsed_schema, value).expect("datum"));
        let event = kafka_event(&json!({
            "orders-0": [{
                "topic": "orders",
                "partition": 0,
                "offset": 20,
                "timestamp": 1_690_900_005_000_i64,
                "timestampType": "CREATE_TIME",
                "key": null,
                "value": encoded,
                "valueSchemaMetadata": {
                    "dataFormat": "AVRO",
                    "schemaId": "avro-value-schema"
                },
                "headers": []
            }]
        }));
        let config = KafkaSchemaConfig::<String, Order>::new()
            .with_value_decoder(AvroKafkaFieldDecoder::new(schema).expect("decoder"));

        let records = KafkaSchemaConsumer::new(config)
            .records(event)
            .expect("record should decode");
        let record = records.records.first().expect("record should exist");

        assert_eq!(
            record.value.as_ref(),
            Some(&Order {
                order_id: "order-1".to_string(),
                quantity: 2,
            })
        );
    }

    #[cfg(feature = "protobuf")]
    #[test]
    fn schema_consumer_decodes_protobuf_values_from_schema_metadata() {
        let order = ProtoOrder {
            order_id: "order-1".to_string(),
            quantity: 2,
        };
        let mut bytes = vec![0];
        bytes.extend(order.encode_to_vec());
        let event = kafka_event(&json!({
            "orders-0": [{
                "topic": "orders",
                "partition": 0,
                "offset": 21,
                "timestamp": 1_690_900_006_000_i64,
                "timestampType": "CREATE_TIME",
                "key": null,
                "value": STANDARD.encode(bytes),
                "valueSchemaMetadata": {
                    "dataFormat": "PROTOBUF",
                    "schemaId": "7d55d475-2244-4485-8341-f74468c1e058"
                },
                "headers": []
            }]
        }));
        let config = KafkaSchemaConfig::<String, ProtoOrder>::from_decoders(
            PrimitiveKafkaFieldDecoder,
            ProtobufKafkaFieldDecoder::from_schema_metadata(),
        );

        let records = KafkaSchemaConsumer::new(config)
            .records(event)
            .expect("record should decode");
        let record = records.records.first().expect("record should exist");

        assert_eq!(record.value.as_ref(), Some(&order));
    }

    fn kafka_event(records: &serde_json::Value) -> KafkaEvent {
        serde_json::from_value(json!({
            "eventSource": "aws:kafka",
            "eventSourceArn": "arn:aws:kafka:us-east-1:123456789012:cluster/orders",
            "bootstrapServers": "b-1.example:9098",
            "records": records
        }))
        .expect("Kafka event should deserialize")
    }
}
