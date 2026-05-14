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
  set operations, optional Secrets Manager, AppConfig, and DynamoDB providers, force-fetch support, and JSON, base64, and
  suffix-based auto transforms
- JMESPath extraction with reusable compiled expressions, typed extraction, common Lambda event envelope constants, and
  Powertools decode functions for JSON, base64, and base64-gzip payloads
- data masking for JSON payloads with whole-value erasure, field masking by JSON Pointer, dot path, and common
  JSONPath-style selectors, fixed/dynamic, custom, and regex masking strategies, per-field masking rules, provider
  encryption/decryption, an optional direct AWS KMS provider, and configurable missing-field behavior
- Kafka consumer record materialization that flattens Lambda Kafka event records, decodes primitive or JSON base64 keys
  and values, supports schema-aware key/value decoder configuration with Event Source Mapping metadata, provides
  optional Avro and Protobuf decode helpers, decodes headers, and preserves original encoded fields
- sync and async seekable streaming over byte-range and AWS SDK-backed S3 object range sources, an `S3Object`
  convenience reader, and optional gzip, CSV, and ZIP transforms
- serde JSON parsing facade with structured parse errors and optional `aws_lambda_events` API Gateway
  REST/HTTP/WebSocket API bodies, API Gateway WebSocket lifecycle aliases, API Gateway Lambda authorizer aliases,
  common Powertools parser event and record model aliases, AppSync Lambda authorizer aliases, AppSync direct
  resolver/batch aliases and arguments/source, AppSync Events publish payload, Bedrock Agent OpenAPI input text,
  ActiveMQ message data, ALB, Lambda Function URL, and VPC Lattice body, SQS, SNS, SNS-over-SQS, RabbitMQ message data,
  S3, S3-over-SQS, S3 Object Lambda configuration payload, S3 Batch job task, EventBridge, CloudFormation custom
  resource aliases/properties, Cognito User Pool user attributes, Cognito User Pool trigger aliases, Auto Scaling,
  AWS Config, CloudWatch Alarms, CodeCommit, CodeDeploy lifecycle hooks, CodePipeline jobs, Connect contact flows, SES,
  CloudWatch Logs, Kinesis, Kinesis-delivered DynamoDB stream image, Firehose, Firehose-delivered SQS,
  Kinesis-delivered CloudWatch Logs, DynamoDB stream image pairs, Kafka envelopes, and Secrets Manager rotation, plus
  Transfer Family authorizer
  event/response, AppSync Events, Bedrock Agent OpenAPI and function-details event models/input text, CloudWatch
  dashboard custom widget event model, DynamoDB stream on-failure destination, S3 EventBridge notification, S3 event
  notification with Intelligent-Tiering support, IoT Core registry, Cognito migrate-user, and Cognito custom sender event
  models
- sequential and concurrent batch record processing, partial batch response builders, stream checkpoint helpers, and
  optional `aws_lambda_events` SQS, Kinesis, and DynamoDB stream adapters with FIFO SQS early-stop behavior and
  parser-integrated SQS message body, Kinesis record data, and DynamoDB stream image processing
- validation helpers for required text, text length, numeric ranges, custom predicates, inbound/outbound value wrappers,
  optional local JSON Schema validation with a compiled schema cache, and optional JMESPath envelope extraction before
  schema validation
- idempotency keys, payload hashing, JSON Pointer and optional JMESPath key extraction, configurable full/subset/no
  payload validation, sync and async handler workflows, replay behavior, Lambda remaining-time in-progress expiry, sync
  and async store traits, an in-memory store, an optional local cache wrapper, an optional DynamoDB store, and buildable
  local/AWS snippets
- feature flag schema parsing, sync/async store traits, in-memory and optional AppConfig stores, boolean and JSON-valued
  evaluation, enabled-feature listing, configuration cache policies, common context comparators, modulo range matching,
  and time-window rules
- tracer configuration, X-Ray header context parsing/rendering, trace segment records, JSON-compatible trace values,
  optional X-Ray-compatible subsegment document rendering, optional X-Ray daemon UDP transport, optional `tracing` span
  integration, optional OpenTelemetry span builder/attribute export, and a buildable OpenTelemetry SDK/stdout exporter
  snippet
- event-handler request/response types, route matching, dynamic path parameters, sync and async router dispatch,
  multi-method route registration, built-in HTTP errors, unsupported-method `405` adapter responses, custom not-found
  and fallible route error handlers, typed fallible route error handlers, router composition with path prefixes,
  router-level and route-specific request/response middleware, request-scoped and shared typed extensions, origin-aware
  CORS handling, optional AppSync GraphQL scalar helpers, optional router-level and route-specific validation hooks,
  optional gzip/deflate compression middleware, and optional API Gateway REST/HTTP/WebSocket API, ALB, Lambda Function
  URL, VPC Lattice, sync/async AppSync direct and batch resolver composition, AppSync Events routing and composition,
  Bedrock Agent OpenAPI adapter, and sync/async Bedrock Agent function-details resolver
- testing helper surfaces for Lambda context stubs, handler harnesses, parameter provider stubs, optional feature flag
  and idempotency store stubs, optional S3 object client stubs, and fixture loading

Not yet implemented: broader `aws_lambda_events` envelopes and fixtures, vendor-specific OpenTelemetry exporter
examples, additional event-handler adapters, and crates.io publishing. See
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
- [Tracer](docs/features/tracer.md)
- [Parameters](docs/features/parameters.md)
- [JMESPath](docs/features/jmespath.md)
- [Data Masking](docs/features/data-masking.md)
- [Kafka](docs/features/kafka.md)
- [Streaming](docs/features/streaming.md)
- [Parser](docs/features/parser.md)
- [Batch](docs/features/batch.md)
- [Idempotency](docs/features/idempotency.md)
- [Validation](docs/features/validation.md)
- [Feature Flags](docs/features/feature-flags.md)
- [Event Handler](docs/features/event-handler.md)
- [Testing](docs/features/testing.md)

## Validation

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo check --workspace --all-targets --all-features
```
