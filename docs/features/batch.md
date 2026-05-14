# Batch

The batch utility helps Lambda handlers process batch records and return partial batch responses for failed records. It
is exposed through the `batch` Cargo feature on the umbrella crate:

```toml
aws-lambda-powertools = { version = "0.1", features = ["batch"] }
```

## Supported Behavior

- `BatchRecord<T>` for dependency-free batch payloads with explicit item identifiers.
- `BatchProcessor` for sequential and scoped-thread concurrent record processing.
- `BatchProcessingReport` for preserving per-record success and failure results in input order.
- `BatchResponse` serialization compatible with the Lambda `batchItemFailures` response shape.
- Stream checkpoint helpers that report the first failed Kinesis or DynamoDB sequence number.
- SQS FIFO early-stop behavior, where all records after the first failed record are reported as failed without being
  processed.
- Optional parser integration for SQS message bodies, Kinesis record data, and DynamoDB stream images, where malformed
  records are reported as batch failures before the record handler runs.
- Optional `aws_lambda_events` adapters for SQS, Kinesis, DynamoDB stream, and Kafka events.

## Source Adapters

Enable `batch-aws-lambda-events` to process event models from the `aws_lambda_events` crate:

```toml
aws-lambda-powertools = { version = "0.1", features = ["batch-aws-lambda-events"] }
```

The SQS adapter uses `message_id` values as failed item identifiers. Kinesis and DynamoDB adapters use sequence numbers
and fall back to event IDs when a sequence number is unavailable. The Kafka adapter returns Lambda's Kafka-specific
topic-partition and offset failure shape through `KafkaBatchResponse`.

Use `process_sqs_fifo` or `process_sqs_fifo_response` for FIFO queues so Lambda retries the first failed record and all
later records in the same batch.

## Parser Integration

Enable `batch-parser` to combine SQS, Kinesis, and DynamoDB stream event adapters with parser-backed payload decoding:

```toml
aws-lambda-powertools = { version = "0.1", features = ["batch-parser"] }
```

Use `process_sqs_message_bodies` when each SQS message body is JSON for a Rust type. Use `process_kinesis_records` when
each Kinesis record data payload is JSON for a Rust type. Use `process_dynamodb_new_images` or
`process_dynamodb_old_images` when DynamoDB stream images should decode into a Rust type. Records with missing SQS
bodies, missing DynamoDB images, malformed JSON, or payloads that do not decode into the target type are reported in the
partial batch response and are not passed to the handler. Kinesis and DynamoDB parse failures also participate in stream
checkpoint helpers through their sequence numbers.

## Snippet

The buildable snippet in [examples/snippets/batch/src/main.rs](../../examples/snippets/batch/src/main.rs) builds a
partial batch response for generic records, for an SQS FIFO event, for parser-integrated SQS message bodies, and for
parser-integrated Kinesis record data and DynamoDB stream images, plus a Kafka partial batch response.

Run it locally with:

```sh
cargo run -p batch-snippet
```
