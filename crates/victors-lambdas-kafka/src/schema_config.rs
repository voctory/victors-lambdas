//! Schema-aware Kafka consumer configuration.

use std::{fmt, sync::Arc};

use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::{
    KafkaConsumerError, KafkaConsumerResult,
    record::{decode_base64_json_field, decode_base64_string_field},
};

/// Kafka record field being decoded.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KafkaField {
    /// Kafka record key.
    Key,
    /// Kafka record value.
    Value,
}

impl KafkaField {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Key => "key",
            Self::Value => "value",
        }
    }
}

/// Supported schema types for Kafka key and value decoding.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KafkaSchemaType {
    /// JSON payloads.
    Json,
    /// Apache Avro payloads.
    Avro,
    /// Protocol Buffer payloads.
    Protobuf,
}

impl KafkaSchemaType {
    /// Returns the Event Source Mapping data format value.
    #[must_use]
    pub const fn data_format(self) -> &'static str {
        match self {
            Self::Json => "JSON",
            Self::Avro => "AVRO",
            Self::Protobuf => "PROTOBUF",
        }
    }
}

/// Schema metadata supplied by Lambda Event Source Mapping Schema Registry integration.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct KafkaSchemaMetadata {
    data_format: Option<String>,
    schema_id: Option<String>,
}

impl KafkaSchemaMetadata {
    /// Creates schema metadata with a data format.
    #[must_use]
    pub fn new(data_format: impl Into<String>) -> Self {
        Self {
            data_format: Some(data_format.into()),
            schema_id: None,
        }
    }

    /// Adds a schema ID.
    #[must_use]
    pub fn with_schema_id(mut self, schema_id: impl Into<String>) -> Self {
        self.schema_id = Some(schema_id.into());
        self
    }

    /// Returns the Event Source Mapping data format.
    #[must_use]
    pub fn data_format(&self) -> Option<&str> {
        self.data_format.as_deref()
    }

    /// Returns the schema ID from Event Source Mapping metadata.
    #[must_use]
    pub fn schema_id(&self) -> Option<&str> {
        self.schema_id.as_deref()
    }

    pub(crate) fn from_value(value: &Value) -> Option<Self> {
        let object = value.as_object()?;
        let data_format = object
            .get("dataFormat")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);
        let schema_id = object
            .get("schemaId")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);

        if data_format.is_none() && schema_id.is_none() {
            return None;
        }

        Some(Self {
            data_format,
            schema_id,
        })
    }
}

/// Decodes a base64 Kafka field into a typed key or value.
pub trait KafkaFieldDecoder<T>: Send + Sync {
    /// Decodes a Kafka field using optional Event Source Mapping schema metadata.
    ///
    /// # Errors
    ///
    /// Returns an error when the field cannot be decoded into `T`.
    fn decode(
        &self,
        encoded: &str,
        metadata: Option<&KafkaSchemaMetadata>,
        field: KafkaField,
    ) -> KafkaConsumerResult<T>;
}

/// Primitive Kafka field decoder.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PrimitiveKafkaFieldDecoder;

impl<T> KafkaFieldDecoder<T> for PrimitiveKafkaFieldDecoder
where
    T: DeserializeOwned,
{
    fn decode(
        &self,
        encoded: &str,
        _metadata: Option<&KafkaSchemaMetadata>,
        field: KafkaField,
    ) -> KafkaConsumerResult<T> {
        let decoded = decode_base64_string_field(field.as_str(), encoded)?;
        serde_json::from_value(serde_json::Value::String(decoded))
            .map_err(|error| KafkaConsumerError::json(field.as_str(), error))
    }
}

/// JSON Kafka field decoder.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct JsonKafkaFieldDecoder;

impl<T> KafkaFieldDecoder<T> for JsonKafkaFieldDecoder
where
    T: DeserializeOwned,
{
    fn decode(
        &self,
        encoded: &str,
        metadata: Option<&KafkaSchemaMetadata>,
        field: KafkaField,
    ) -> KafkaConsumerResult<T> {
        validate_data_format(field, KafkaSchemaType::Json, metadata)?;
        decode_base64_json_field(field.as_str(), encoded)
    }
}

/// Avro Kafka field decoder.
#[cfg(feature = "avro")]
#[cfg_attr(docsrs, doc(cfg(feature = "avro")))]
#[derive(Clone, Debug, PartialEq)]
pub struct AvroKafkaFieldDecoder {
    schema: apache_avro::Schema,
}

#[cfg(feature = "avro")]
impl AvroKafkaFieldDecoder {
    /// Parses an Avro schema string for Kafka field decoding.
    ///
    /// # Errors
    ///
    /// Returns an error when the Avro schema is invalid.
    pub fn new(schema: &str) -> KafkaConsumerResult<Self> {
        apache_avro::Schema::parse_str(schema)
            .map(Self::from_schema)
            .map_err(|error| KafkaConsumerError::schema("schema", "Avro", error))
    }

    /// Creates an Avro decoder from a parsed schema.
    #[must_use]
    pub const fn from_schema(schema: apache_avro::Schema) -> Self {
        Self { schema }
    }

    /// Returns the parsed Avro schema.
    #[must_use]
    pub const fn schema(&self) -> &apache_avro::Schema {
        &self.schema
    }
}

#[cfg(feature = "avro")]
impl<T> KafkaFieldDecoder<T> for AvroKafkaFieldDecoder
where
    T: DeserializeOwned,
{
    fn decode(
        &self,
        encoded: &str,
        metadata: Option<&KafkaSchemaMetadata>,
        field: KafkaField,
    ) -> KafkaConsumerResult<T> {
        validate_data_format(field, KafkaSchemaType::Avro, metadata)?;
        crate::schema::decode_base64_avro_field(field.as_str(), encoded, &self.schema)
    }
}

/// Protobuf Kafka field decoder.
#[cfg(feature = "protobuf")]
#[cfg_attr(docsrs, doc(cfg(feature = "protobuf")))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProtobufKafkaFieldDecoder {
    wire_format: Option<crate::ProtobufWireFormat>,
}

#[cfg(feature = "protobuf")]
impl ProtobufKafkaFieldDecoder {
    /// Creates a decoder that infers registry framing from schema metadata.
    #[must_use]
    pub const fn from_schema_metadata() -> Self {
        Self { wire_format: None }
    }

    /// Creates a decoder with an explicit Protobuf wire format.
    #[must_use]
    pub const fn with_wire_format(wire_format: crate::ProtobufWireFormat) -> Self {
        Self {
            wire_format: Some(wire_format),
        }
    }

    /// Returns the explicit wire format, if one is configured.
    #[must_use]
    pub const fn wire_format(&self) -> Option<crate::ProtobufWireFormat> {
        self.wire_format
    }
}

#[cfg(feature = "protobuf")]
impl Default for ProtobufKafkaFieldDecoder {
    fn default() -> Self {
        Self::from_schema_metadata()
    }
}

#[cfg(feature = "protobuf")]
impl<M> KafkaFieldDecoder<M> for ProtobufKafkaFieldDecoder
where
    M: prost::Message + Default,
{
    fn decode(
        &self,
        encoded: &str,
        metadata: Option<&KafkaSchemaMetadata>,
        field: KafkaField,
    ) -> KafkaConsumerResult<M> {
        validate_data_format(field, KafkaSchemaType::Protobuf, metadata)?;
        let wire_format = self
            .wire_format
            .unwrap_or_else(|| protobuf_wire_format_from_metadata(metadata));
        crate::schema::decode_base64_protobuf_with_format_field(
            field.as_str(),
            encoded,
            wire_format,
        )
    }
}

#[cfg(feature = "protobuf")]
fn protobuf_wire_format_from_metadata(
    metadata: Option<&KafkaSchemaMetadata>,
) -> crate::ProtobufWireFormat {
    let Some(schema_id) = metadata.and_then(KafkaSchemaMetadata::schema_id) else {
        return crate::ProtobufWireFormat::Plain;
    };

    if schema_id.len() > 20 {
        crate::ProtobufWireFormat::GlueSchemaRegistry
    } else {
        crate::ProtobufWireFormat::ConfluentSchemaRegistry
    }
}

/// Schema-aware Kafka key and value decoding configuration.
#[derive(Clone)]
pub struct KafkaSchemaConfig<K, V> {
    key_decoder: Arc<dyn KafkaFieldDecoder<K>>,
    value_decoder: Arc<dyn KafkaFieldDecoder<V>>,
}

impl<K, V> fmt::Debug for KafkaSchemaConfig<K, V> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("KafkaSchemaConfig")
            .finish_non_exhaustive()
    }
}

impl<K, V> KafkaSchemaConfig<K, V> {
    /// Creates a schema config from explicit key and value decoders.
    #[must_use]
    pub fn from_decoders<KD, VD>(key_decoder: KD, value_decoder: VD) -> Self
    where
        KD: KafkaFieldDecoder<K> + 'static,
        VD: KafkaFieldDecoder<V> + 'static,
    {
        Self {
            key_decoder: Arc::new(key_decoder),
            value_decoder: Arc::new(value_decoder),
        }
    }

    /// Sets the key decoder.
    #[must_use]
    pub fn with_key_decoder<D>(mut self, decoder: D) -> Self
    where
        D: KafkaFieldDecoder<K> + 'static,
    {
        self.key_decoder = Arc::new(decoder);
        self
    }

    /// Sets the value decoder.
    #[must_use]
    pub fn with_value_decoder<D>(mut self, decoder: D) -> Self
    where
        D: KafkaFieldDecoder<V> + 'static,
    {
        self.value_decoder = Arc::new(decoder);
        self
    }

    pub(crate) fn key_decoder(&self) -> &dyn KafkaFieldDecoder<K> {
        self.key_decoder.as_ref()
    }

    pub(crate) fn value_decoder(&self) -> &dyn KafkaFieldDecoder<V> {
        self.value_decoder.as_ref()
    }
}

impl<K, V> KafkaSchemaConfig<K, V>
where
    K: DeserializeOwned + 'static,
    V: DeserializeOwned + 'static,
{
    /// Creates a schema config that decodes keys and values as primitive `UTF-8` text.
    #[must_use]
    pub fn new() -> Self {
        Self::from_decoders(PrimitiveKafkaFieldDecoder, PrimitiveKafkaFieldDecoder)
    }

    /// Creates a schema config that decodes Kafka values as JSON.
    #[must_use]
    pub fn json_values() -> Self {
        Self::new().with_value_decoder(JsonKafkaFieldDecoder)
    }

    /// Creates a schema config that decodes Kafka keys and values as JSON.
    #[must_use]
    pub fn json_key_and_value() -> Self {
        Self::from_decoders(JsonKafkaFieldDecoder, JsonKafkaFieldDecoder)
    }
}

impl<K, V> Default for KafkaSchemaConfig<K, V>
where
    K: DeserializeOwned + 'static,
    V: DeserializeOwned + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

fn validate_data_format(
    field: KafkaField,
    expected: KafkaSchemaType,
    metadata: Option<&KafkaSchemaMetadata>,
) -> KafkaConsumerResult<()> {
    let Some(data_format) = metadata.and_then(KafkaSchemaMetadata::data_format) else {
        return Ok(());
    };

    if data_format.eq_ignore_ascii_case(expected.data_format()) {
        return Ok(());
    }

    Err(KafkaConsumerError::schema(
        field.as_str(),
        expected.data_format(),
        format!(
            "expected data format {}, got {data_format}",
            expected.data_format()
        ),
    ))
}
