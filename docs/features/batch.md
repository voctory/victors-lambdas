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
- Optional parser integration for SQS message bodies, where malformed records are reported as batch failures before the
  record handler runs.
- Optional `aws_lambda_events` adapters for SQS, Kinesis, and DynamoDB stream events.

## Source Adapters

Enable `batch-aws-lambda-events` to process event models from the `aws_lambda_events` crate:

```toml
aws-lambda-powertools = { version = "0.1", features = ["batch-aws-lambda-events"] }
```

The SQS adapter uses `message_id` values as failed item identifiers. Kinesis and DynamoDB adapters use sequence numbers
and fall back to event IDs when a sequence number is unavailable.

Use `process_sqs_fifo` or `process_sqs_fifo_response` for FIFO queues so Lambda retries the first failed record and all
later records in the same batch.

## Parser Integration

Enable `batch-parser` to combine SQS event adapters with the parser utility:

```toml
aws-lambda-powertools = { version = "0.1", features = ["batch-parser"] }
```

Use `process_sqs_message_bodies` when each SQS message body is JSON for a Rust type. Records with missing bodies,
malformed JSON, or payloads that do not decode into the target type are reported in the partial batch response and are
not passed to the handler.

## Snippet

The buildable snippet in [examples/snippets/batch/src/main.rs](../../examples/snippets/batch/src/main.rs) builds a
partial batch response for generic records, for an SQS FIFO event, and for parser-integrated SQS message bodies.

Run it locally with:

```sh
cargo run -p batch-snippet
```
