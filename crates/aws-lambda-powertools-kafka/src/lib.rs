//! Kafka consumer record utility.

mod config;
mod error;
mod record;
mod schema;

pub use config::{KafkaConsumerConfig, KafkaFieldDeserializer};
pub use error::{KafkaConsumerError, KafkaConsumerErrorKind, KafkaConsumerResult};
pub use record::{
    ConsumerRecord, ConsumerRecords, KafkaConsumer, consumer_records, decode_base64_json,
    decode_base64_string,
};

#[cfg(feature = "avro")]
pub use schema::decode_base64_avro;

#[cfg(feature = "protobuf")]
pub use schema::{ProtobufWireFormat, decode_base64_protobuf, decode_base64_protobuf_with_format};
