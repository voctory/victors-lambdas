//! Kafka consumer configuration.

/// Deserializer used for a Kafka record field.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum KafkaFieldDeserializer {
    /// Decode base64 bytes as UTF-8 text.
    #[default]
    Primitive,
    /// Decode base64 bytes as JSON and then deserialize into the target type.
    Json,
}

/// Configuration for deserializing Kafka record keys and values.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct KafkaConsumerConfig {
    key_deserializer: KafkaFieldDeserializer,
    value_deserializer: KafkaFieldDeserializer,
}

impl KafkaConsumerConfig {
    /// Creates a config that decodes keys and values as primitive UTF-8 text.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            key_deserializer: KafkaFieldDeserializer::Primitive,
            value_deserializer: KafkaFieldDeserializer::Primitive,
        }
    }

    /// Creates a config that decodes Kafka values as JSON.
    #[must_use]
    pub const fn json_values() -> Self {
        Self::new().with_value_deserializer(KafkaFieldDeserializer::Json)
    }

    /// Creates a config that decodes Kafka keys and values as JSON.
    #[must_use]
    pub const fn json_key_and_value() -> Self {
        Self::new()
            .with_key_deserializer(KafkaFieldDeserializer::Json)
            .with_value_deserializer(KafkaFieldDeserializer::Json)
    }

    /// Sets the key deserializer.
    #[must_use]
    pub const fn with_key_deserializer(mut self, deserializer: KafkaFieldDeserializer) -> Self {
        self.key_deserializer = deserializer;
        self
    }

    /// Sets the value deserializer.
    #[must_use]
    pub const fn with_value_deserializer(mut self, deserializer: KafkaFieldDeserializer) -> Self {
        self.value_deserializer = deserializer;
        self
    }

    /// Returns the key deserializer.
    #[must_use]
    pub const fn key_deserializer(&self) -> KafkaFieldDeserializer {
        self.key_deserializer
    }

    /// Returns the value deserializer.
    #[must_use]
    pub const fn value_deserializer(&self) -> KafkaFieldDeserializer {
        self.value_deserializer
    }
}
