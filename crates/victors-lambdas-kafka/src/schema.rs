//! Schema-backed Kafka field deserializers.

#[cfg(any(feature = "avro", feature = "protobuf"))]
use base64::{Engine as _, engine::general_purpose::STANDARD};

#[cfg(any(feature = "avro", feature = "protobuf"))]
use crate::{KafkaConsumerError, KafkaConsumerResult};

/// Decodes base64 bytes as an Avro datum using a parsed Avro schema.
///
/// # Errors
///
/// Returns an error when the input is not valid base64, the bytes are not valid for the schema, or
/// the Avro value cannot be deserialized into `T`.
#[cfg(feature = "avro")]
#[cfg_attr(docsrs, doc(cfg(feature = "avro")))]
pub fn decode_base64_avro<T>(encoded: &str, schema: &apache_avro::Schema) -> KafkaConsumerResult<T>
where
    T: serde::de::DeserializeOwned,
{
    decode_base64_avro_field("value", encoded, schema)
}

#[cfg(feature = "avro")]
pub(crate) fn decode_base64_avro_field<T>(
    field: &'static str,
    encoded: &str,
    schema: &apache_avro::Schema,
) -> KafkaConsumerResult<T>
where
    T: serde::de::DeserializeOwned,
{
    let bytes = decode_base64_schema_field(field, encoded)?;
    let mut reader = bytes.as_slice();
    let value = apache_avro::from_avro_datum(schema, &mut reader, None)
        .map_err(|error| KafkaConsumerError::schema(field, "Avro", error))?;

    apache_avro::from_value::<T>(&value)
        .map_err(|error| KafkaConsumerError::schema(field, "Avro", error))
}

/// Schema registry framing used by a Protobuf Kafka field.
#[cfg(feature = "protobuf")]
#[cfg_attr(docsrs, doc(cfg(feature = "protobuf")))]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ProtobufWireFormat {
    /// Plain Protobuf bytes with no registry prefix.
    #[default]
    Plain,
    /// AWS Glue Schema Registry framing, where the first byte is skipped.
    GlueSchemaRegistry,
    /// Confluent Schema Registry message-index framing.
    ConfluentSchemaRegistry,
}

/// Decodes base64 bytes as a Protobuf message with no schema registry framing.
///
/// # Errors
///
/// Returns an error when the input is not valid base64 or the bytes cannot be decoded into `M`.
#[cfg(feature = "protobuf")]
#[cfg_attr(docsrs, doc(cfg(feature = "protobuf")))]
pub fn decode_base64_protobuf<M>(encoded: &str) -> KafkaConsumerResult<M>
where
    M: prost::Message + Default,
{
    decode_base64_protobuf_with_format(encoded, ProtobufWireFormat::Plain)
}

/// Decodes base64 bytes as a Protobuf message with optional schema registry framing.
///
/// # Errors
///
/// Returns an error when the input is not valid base64, registry framing cannot be removed, or the
/// remaining bytes cannot be decoded into `M`.
#[cfg(feature = "protobuf")]
#[cfg_attr(docsrs, doc(cfg(feature = "protobuf")))]
pub fn decode_base64_protobuf_with_format<M>(
    encoded: &str,
    format: ProtobufWireFormat,
) -> KafkaConsumerResult<M>
where
    M: prost::Message + Default,
{
    decode_base64_protobuf_with_format_field("value", encoded, format)
}

#[cfg(feature = "protobuf")]
pub(crate) fn decode_base64_protobuf_with_format_field<M>(
    field: &'static str,
    encoded: &str,
    format: ProtobufWireFormat,
) -> KafkaConsumerResult<M>
where
    M: prost::Message + Default,
{
    let bytes = decode_base64_schema_field(field, encoded)?;
    let payload = match format {
        ProtobufWireFormat::Plain => bytes,
        ProtobufWireFormat::GlueSchemaRegistry => bytes
            .get(1..)
            .ok_or_else(|| {
                KafkaConsumerError::schema(field, "Protobuf", "missing Glue schema registry prefix")
            })?
            .to_vec(),
        ProtobufWireFormat::ConfluentSchemaRegistry => {
            remove_confluent_message_indexes(field, &bytes)?
        }
    };

    M::decode(payload.as_slice())
        .map_err(|error| KafkaConsumerError::schema(field, "Protobuf", error))
}

#[cfg(any(feature = "avro", feature = "protobuf"))]
pub(crate) fn decode_base64_schema_field(
    field: &'static str,
    encoded: &str,
) -> KafkaConsumerResult<Vec<u8>> {
    STANDARD
        .decode(encoded)
        .map_err(|error| KafkaConsumerError::base64(field, error))
}

#[cfg(feature = "protobuf")]
fn remove_confluent_message_indexes(
    field: &'static str,
    bytes: &[u8],
) -> KafkaConsumerResult<Vec<u8>> {
    let (index_count, mut position) = decode_unsigned_varint(field, bytes)?;
    for _ in 0..index_count {
        let (_, next_position) = decode_unsigned_varint(field, &bytes[position..])?;
        position += next_position;
    }

    Ok(bytes[position..].to_vec())
}

#[cfg(feature = "protobuf")]
fn decode_unsigned_varint(field: &'static str, bytes: &[u8]) -> KafkaConsumerResult<(u64, usize)> {
    let mut value = 0_u64;

    for (index, byte) in bytes.iter().enumerate() {
        let shift = u32::try_from(index)
            .ok()
            .and_then(|index| index.checked_mul(7))
            .ok_or_else(|| KafkaConsumerError::schema(field, "Protobuf", "varint is too long"))?;

        if shift >= u64::BITS {
            return Err(KafkaConsumerError::schema(
                field,
                "Protobuf",
                "varint is too long",
            ));
        }

        value |= u64::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Ok((value, index + 1));
        }
    }

    Err(KafkaConsumerError::schema(
        field,
        "Protobuf",
        "unterminated varint",
    ))
}

#[cfg(test)]
mod tests {
    use base64::Engine as _;

    #[cfg(feature = "avro")]
    use serde::Deserialize;

    #[cfg(feature = "protobuf")]
    use prost::Message;

    use super::*;

    #[cfg(feature = "avro")]
    #[derive(Debug, Deserialize, PartialEq)]
    struct AvroOrder {
        order_id: String,
        quantity: i32,
    }

    #[cfg(feature = "avro")]
    #[test]
    fn decodes_avro_payloads() {
        let schema = apache_avro::Schema::parse_str(
            r#"{
                "type": "record",
                "name": "Order",
                "fields": [
                    {"name": "order_id", "type": "string"},
                    {"name": "quantity", "type": "int"}
                ]
            }"#,
        )
        .expect("schema should parse");
        let value = apache_avro::types::Value::Record(vec![
            (
                "order_id".to_string(),
                apache_avro::types::Value::String("order-1".to_string()),
            ),
            ("quantity".to_string(), apache_avro::types::Value::Int(2)),
        ]);
        let bytes = apache_avro::to_avro_datum(&schema, value).expect("datum should encode");
        let encoded = STANDARD.encode(bytes);

        let order: AvroOrder = decode_base64_avro(&encoded, &schema).expect("datum should decode");

        assert_eq!(
            order,
            AvroOrder {
                order_id: "order-1".to_string(),
                quantity: 2,
            }
        );
    }

    #[cfg(feature = "protobuf")]
    #[derive(Clone, PartialEq, prost::Message)]
    struct ProtoOrder {
        #[prost(string, tag = "1")]
        order_id: String,
        #[prost(uint32, tag = "2")]
        quantity: u32,
    }

    #[cfg(feature = "protobuf")]
    #[test]
    fn decodes_plain_protobuf_payloads() {
        let order = ProtoOrder {
            order_id: "order-1".to_string(),
            quantity: 2,
        };
        let encoded = STANDARD.encode(order.encode_to_vec());

        let decoded: ProtoOrder = decode_base64_protobuf(&encoded).expect("protobuf should decode");

        assert_eq!(decoded, order);
    }

    #[cfg(feature = "protobuf")]
    #[test]
    fn decodes_glue_schema_registry_protobuf_payloads() {
        let order = ProtoOrder {
            order_id: "order-1".to_string(),
            quantity: 2,
        };
        let mut bytes = vec![0];
        bytes.extend(order.encode_to_vec());
        let encoded = STANDARD.encode(bytes);

        let decoded: ProtoOrder =
            decode_base64_protobuf_with_format(&encoded, ProtobufWireFormat::GlueSchemaRegistry)
                .expect("protobuf should decode");

        assert_eq!(decoded, order);
    }

    #[cfg(feature = "protobuf")]
    #[test]
    fn decodes_confluent_schema_registry_protobuf_payloads() {
        let order = ProtoOrder {
            order_id: "order-1".to_string(),
            quantity: 2,
        };
        let mut bytes = vec![0];
        bytes.extend(order.encode_to_vec());
        let encoded = STANDARD.encode(bytes);

        let decoded: ProtoOrder = decode_base64_protobuf_with_format(
            &encoded,
            ProtobufWireFormat::ConfluentSchemaRegistry,
        )
        .expect("protobuf should decode");

        assert_eq!(decoded, order);
    }
}
