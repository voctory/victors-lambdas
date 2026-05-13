# Powertools Lambda Rust

Powertools Lambda Rust is an unofficial Rust toolkit for AWS Lambda functions. It is early-stage; treat APIs as
pre-release.

The workspace currently contains the umbrella crate `aws-lambda-powertools`, feature-gated utility crates, a
`basic-lambda` workspace example, buildable snippets, and CI for formatting, linting, tests, and workspace checks. The
first implementation tranche has landed:

- shared service config, environment parsing helpers, cold-start tracking, and runtime metadata
- structured JSON logging with levels, persistent and per-entry fields, optional event rendering, debug sampling,
  correlation IDs, Lambda context fields, key redaction, custom formatter/redaction hooks, optional `tracing`
  subscriber integration, and stdout emission
- CloudWatch EMF JSON rendering with metrics, dimensions, default dimensions, metadata, validation, limits,
  high-resolution metrics, stdout flushing, explicit timestamps, overflow flush helpers, async capture helpers, and
  cold-start metric support
- sync and async parameter provider/cache traits with in-memory, optional SSM Parameter Store single, by-name, path, and
  set operations, optional Secrets Manager, AppConfig, and DynamoDB providers, force-fetch support, and JSON/base64
  transforms
- serde JSON parsing facade with structured parse errors and optional `aws_lambda_events` API Gateway
  REST/HTTP/WebSocket API, AppSync direct resolver arguments/source, Bedrock Agent input text, ALB, Lambda Function URL,
  and VPC Lattice body, SQS, SNS, SNS-over-SQS, S3, S3-over-SQS, S3 Object Lambda configuration payload, S3 Batch job
  task, EventBridge, CloudFormation custom resource properties, Cognito User Pool user attributes, SES, CloudWatch Logs,
  Kinesis, Firehose, DynamoDB stream image, and Kafka envelopes, plus Transfer Family authorizer event and response
  models
- sequential and concurrent batch record processing, partial batch response builders, stream checkpoint helpers, and
  optional `aws_lambda_events` SQS, Kinesis, and DynamoDB stream adapters with FIFO SQS early-stop behavior
- validation helpers for required text, text length, numeric ranges, custom predicates, inbound/outbound value wrappers,
  and optional local JSON Schema validation with a compiled schema cache
- idempotency keys, payload hashing, JSON Pointer key extraction, sync and async handler workflows, replay behavior,
  Lambda remaining-time in-progress expiry, sync and async store traits, an in-memory store, and an optional DynamoDB
  store
- feature flag schema parsing, sync/async store traits, in-memory and optional AppConfig stores, boolean and JSON-valued
  evaluation, enabled-feature listing, configuration cache policies, common context comparators, modulo range matching,
  and time-window rules
- tracer configuration, X-Ray header context parsing, trace segment records, JSON-compatible trace values, and optional
  `tracing` span integration
- event-handler request/response types, route matching, dynamic path parameters, sync and async router dispatch,
  request/response middleware, CORS handling, optional validation hooks, optional gzip/deflate compression middleware,
  and optional API Gateway REST/HTTP/WebSocket API, ALB, Lambda Function URL, VPC Lattice, AppSync direct resolver, and
  Bedrock Agent adapters
- testing helper surfaces for Lambda context stubs, parameter provider stubs, and fixture loading

Not yet implemented: broader `aws_lambda_events` envelopes and fixtures, richer idempotency examples, OpenTelemetry or
X-Ray tracing integration, additional event-handler adapters, crates.io publishing, and remaining feature docs. See
[docs/porting-plan.md](docs/porting-plan.md) for the current backlog.

## Workspace

The root is a virtual Cargo workspace using Rust 2024, resolver `3`, and Rust `1.85.0` as the current MSRV. Keep
`Cargo.lock` committed because examples are part of the workspace validation.

Use the umbrella crate with explicit feature flags:

```toml
aws-lambda-powertools = { version = "0.1", features = ["logger", "metrics"] }
```

Local examples use path dependencies until crates are published.

Initial feature docs:

- [Logger](docs/features/logger.md)
- [Metrics](docs/features/metrics.md)
- [Feature Flags](docs/features/feature-flags.md)

## Validation

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo check --workspace --all-targets --all-features
```
