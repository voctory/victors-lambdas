//! Kafka consumer record utility.

mod config;
mod error;
mod record;
mod schema;
mod schema_config;

pub use config::{KafkaConsumerConfig, KafkaFieldDeserializer};
pub use error::{KafkaConsumerError, KafkaConsumerErrorKind, KafkaConsumerResult};
pub use record::{
    ConsumerRecord, ConsumerRecords, KafkaConsumer, KafkaSchemaConsumer, consumer_records,
    decode_base64_json, decode_base64_string, schema_consumer_records,
};
pub use schema_config::{
    JsonKafkaFieldDecoder, KafkaField, KafkaFieldDecoder, KafkaSchemaConfig, KafkaSchemaMetadata,
    KafkaSchemaType, PrimitiveKafkaFieldDecoder,
};

#[cfg(feature = "avro")]
pub use schema::decode_base64_avro;
#[cfg(feature = "avro")]
pub use schema_config::AvroKafkaFieldDecoder;

#[cfg(feature = "protobuf")]
pub use schema::{ProtobufWireFormat, decode_base64_protobuf, decode_base64_protobuf_with_format};
#[cfg(feature = "protobuf")]
pub use schema_config::ProtobufKafkaFieldDecoder;
