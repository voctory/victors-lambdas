# Powertools Lambda Rust

Powertools Lambda Rust is an unofficial Rust toolkit for AWS Lambda functions. It is early-stage; treat APIs as
pre-release.

The workspace currently contains the umbrella crate `aws-lambda-powertools`, feature-gated utility crates, a
`basic-lambda` workspace example, and CI for formatting, linting, tests, and workspace checks. The first implementation
tranche has landed:

- shared service config, environment parsing helpers, cold-start tracking, and runtime metadata
- structured JSON logging with levels, persistent and per-entry fields, optional event rendering, debug sampling,
  correlation IDs, Lambda context fields, key redaction, and stdout emission
- CloudWatch EMF JSON rendering with metrics, dimensions, default dimensions, metadata, validation, limits,
  high-resolution metrics, stdout flushing, and cold-start metric support
- parameter provider/cache traits with an in-memory provider
- serde JSON parsing facade with structured parse errors
- batch record processing and partial batch response builders
- validation helpers for required text, text length, numeric ranges, and custom predicates
- idempotency keys, records, status values, store traits, and an in-memory store
- tracer configuration, X-Ray header context parsing, trace segment records, and JSON-compatible trace values
- dependency-free event-handler request/response types, route matching, dynamic path parameters, and router dispatch
- minimal testing helper surfaces

Not yet implemented: AWS SDK-backed parameter providers, DynamoDB-backed idempotency, `aws_lambda_events` envelopes,
OpenTelemetry or X-Ray tracing integration, API Gateway adapters, JSON Schema validation, crates.io publishing, and full
feature docs. See [docs/porting-plan.md](docs/porting-plan.md) for the current backlog.

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
