# Kafka

The Kafka utility turns Lambda Kafka events into flattened records with decoded keys, values, and headers. It is exposed
through the `kafka` Cargo feature on the umbrella crate:

```toml
aws-lambda-powertools = { version = "0.1", features = ["kafka"] }
```

## Supported Behavior

- Flatten `aws_lambda_events::event::kafka::KafkaEvent` records from topic-partition groups into a single record list.
- Preserve original base64-encoded keys and values alongside decoded fields.
- Decode primitive keys and values as base64 `UTF-8` text.
- Decode `JSON` keys and values into caller-provided Rust types.
- Decode Kafka headers from byte arrays into `UTF-8` strings while preserving the original byte arrays.
- Optional Avro base64 datum decoding behind `kafka-avro`.
- Optional Protobuf base64 message decoding behind `kafka-protobuf`, with plain, AWS Glue Schema Registry, and Confluent
  message-index framing helpers.

Schema registry-backed consumer configuration is not implemented yet; schema helpers can be used on individual record
fields after materialization.

## Snippet

The buildable snippet in [examples/snippets/kafka/src/main.rs](../../examples/snippets/kafka/src/main.rs) decodes a
Kafka event with primitive keys and `JSON` values.

Run it locally with:

```sh
cargo run -p kafka-snippet
```
