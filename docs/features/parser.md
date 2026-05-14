# Parser

The parser utility decodes JSON payloads and selected Lambda event envelopes into Rust types. It is exposed through the
`parser` Cargo feature on the umbrella crate:

```toml
aws-lambda-powertools = { version = "0.1", features = ["parser"] }
```

## Supported Behavior

- `EventParser` facade for parsing JSON strings, bytes, and `serde_json::Value` payloads.
- `ParsedEvent<T>` wrapper for carrying typed payloads through handler code.
- Structured `ParseError` values with error kind, line, column, and message accessors where JSON supplies that context.
- Rust-native event models for Transfer Family authorizers, AppSync Events, Bedrock Agent OpenAPI and function-details
  action groups, DynamoDB stream on-failure destinations, S3 EventBridge notifications, IoT Core registry events, and
  selected Cognito triggers.
- Optional `aws_lambda_events` aliases for API Gateway Lambda authorizer TOKEN, REST API REQUEST, HTTP API payload
  format 1.0, HTTP API payload format 2.0, IAM policy response, and simple response models.
- Optional `aws_lambda_events` envelopes for common payload extraction paths, including API Gateway bodies, AppSync
  resolver arguments/source, AppSync Events publish payloads, Bedrock Agent input text, ActiveMQ message data, ALB
  bodies, Lambda Function URL bodies, VPC Lattice bodies, EventBridge detail, CloudFormation custom resource properties,
  Cognito user attributes, SQS bodies, SNS messages, SNS-over-SQS messages, RabbitMQ message data, S3 records,
  S3-over-SQS records, S3 Object Lambda configuration payloads, S3 Batch tasks, SES records, CloudWatch Logs messages,
  Kinesis records, Firehose records, DynamoDB stream images, and Kafka record values.
- EventBridge Scheduler empty-detail compatibility: Scheduler events with `source` set to `aws.scheduler` and `detail`
  set to the string `"{}"` parse as an empty JSON object.

## Envelopes

Enable `parser-aws-lambda-events` to parse payloads embedded in event models from the `aws_lambda_events` crate:

```toml
aws-lambda-powertools = { version = "0.1", features = ["parser-aws-lambda-events"] }
```

Envelope methods consume the owning event model and return typed payloads. For batch-like sources such as SQS, SNS,
Kinesis, Firehose, DynamoDB streams, and Kafka, the parser returns one parsed payload per record while preserving input
order or the source grouping used by the Lambda event.

## Validation

Parser uses `serde` for structural decoding. Use the `validation` or `validation-jsonschema` features when a handler
also needs business-rule validation, JSON Schema validation, or explicit inbound/outbound validation steps after
decoding.

## Snippet

The buildable snippet in [examples/snippets/parser/src/main.rs](../../examples/snippets/parser/src/main.rs) parses a raw
JSON payload, extracts typed records from an SQS event envelope, and parses the `inputText` field from a Bedrock Agent
OpenAPI event model.

Run it locally with:

```sh
cargo run -p parser-snippet
```
