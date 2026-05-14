//! Kafka consumer record utility.

mod config;
mod error;
mod record;

pub use config::{KafkaConsumerConfig, KafkaFieldDeserializer};
pub use error::{KafkaConsumerError, KafkaConsumerErrorKind, KafkaConsumerResult};
pub use record::{
    ConsumerRecord, ConsumerRecords, KafkaConsumer, consumer_records, decode_base64_json,
    decode_base64_string,
};
