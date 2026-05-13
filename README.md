# Powertools Lambda Rust

Powertools Lambda Rust is an unofficial Rust toolkit for AWS Lambda functions. It is early-stage; treat APIs as
pre-release.

The workspace currently contains the umbrella crate `aws-lambda-powertools`, feature-gated utility crates, a
`basic-lambda` workspace example, and CI for formatting, linting, tests, and workspace checks. The first implementation
tranche has landed:

- shared service config, environment parsing helpers, cold-start tracking, and runtime metadata
- structured JSON logging with levels, persistent and per-entry fields, optional event rendering, debug sampling,
  correlation IDs, Lambda context fields, key redaction, custom formatter/redaction hooks, and stdout emission
- CloudWatch EMF JSON rendering with metrics, dimensions, default dimensions, metadata, validation, limits,
  high-resolution metrics, stdout flushing, explicit timestamps, overflow flush helpers, and cold-start metric support
- parameter provider/cache traits with an in-memory provider
- serde JSON parsing facade with structured parse errors and optional `aws_lambda_events` API Gateway body, SQS, SNS,
  and EventBridge envelopes
- batch record processing, partial batch response builders, and optional `aws_lambda_events` SQS, Kinesis, and DynamoDB
  stream adapters with FIFO SQS early-stop behavior
- validation helpers for required text, text length, numeric ranges, custom predicates, inbound/outbound value wrappers,
  and optional local JSON Schema validation with a compiled schema cache
- idempotency keys, payload hashing, JSON Pointer key extraction, handler workflow, replay behavior, store traits, and
  an in-memory store
- tracer configuration, X-Ray header context parsing, trace segment records, and JSON-compatible trace values
- event-handler request/response types, route matching, dynamic path parameters, router dispatch, request/response
  middleware, CORS handling, and optional API Gateway REST/HTTP API adapters
- minimal testing helper surfaces

Not yet implemented: AWS SDK-backed parameter providers, DynamoDB-backed idempotency persistence, broader
`aws_lambda_events` envelopes and fixtures, OpenTelemetry or X-Ray tracing integration, additional event-handler
adapters, crates.io publishing, and full feature docs. See [docs/porting-plan.md](docs/porting-plan.md) for the
current backlog.

## Workspace

The root is a virtual Cargo workspace using Rust 2024, resolver `3`, and Rust `1.85.0` as the current MSRV. Keep
`Cargo.lock` committed because examples are part of the workspace validation.

Use the umbrella crate with explicit feature flags:

```toml
aws-lambda-powertools = { version = "0.1", features = ["logger", "metrics"] }
```

Local examples use path dependencies until crates are published.

## Validation

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo check --workspace --all-targets --all-features
```
