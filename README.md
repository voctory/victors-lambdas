# Powertools Lambda Rust

Powertools Lambda Rust is an unofficial side project that brings Powertools-style utilities to Rust Lambda functions.
It is not affiliated with, endorsed by, sponsored by, or owned by Amazon Web Services, AWS, or Amazon.com, Inc.

The project is early-stage and APIs should be treated as pre-release. The name describes compatibility goals with
existing Powertools conventions; it does not imply an official AWS project.

## What Is Included

The workspace contains one umbrella crate, `aws-lambda-powertools`, plus focused utility crates under `crates/`.
Utilities are exposed through Cargo features so applications can opt into only the dependencies they need.

Current utility areas:

- logging, metrics, tracing, and Lambda execution metadata
- parameters, idempotency, validation, feature flags, JMESPath, and data masking
- event parsing, event handling, batch processing, Kafka, streaming, and testing helpers

See [docs/porting-plan.md](docs/porting-plan.md) for the detailed feature inventory and backlog.

## Quick Start

The root is a virtual Cargo workspace using Rust 2024, resolver `3`, and Rust `1.85.0` as the current MSRV.
`Cargo.lock` is committed because examples are part of workspace validation.

Use the umbrella crate with explicit feature flags:

```toml
aws-lambda-powertools = { version = "0.1", features = ["logger", "metrics"] }
```

Local examples use path dependencies until crates are published.

## Feature Docs

- [Logger](docs/features/logger.md)
- [Lambda Metadata](docs/features/metadata.md)
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
