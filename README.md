# Victor's Lambdas

Victor's Lambdas is an unofficial side project that provides operational utilities for Rust Lambda functions.
It is not affiliated with, endorsed by, sponsored by, or owned by Amazon Web Services, AWS, or Amazon.com, Inc.

The project is early-stage and APIs should be treated as pre-release. Some APIs preserve familiar cross-language
utility conventions; that does not imply an official AWS project.

## What Is Included

The workspace contains one umbrella crate, `victors-lambdas`, plus focused utility crates under `crates/`.
Utilities are exposed through Cargo features so applications can opt into only the dependencies they need.

Current utility areas:

- logging, metrics, tracing, and Lambda execution metadata
- parameters, idempotency, validation, feature flags, JMESPath, and data masking
- event parsing, event handling, batch processing, Kafka, streaming, and testing helpers

See the
[porting plan](https://github.com/voctory/victors-lambdas/blob/main/docs/porting-plan.md)
for the detailed feature inventory and backlog.

## Quick Start

The root is a virtual Cargo workspace using Rust 2024, resolver `3`, and Rust `1.85.0` as the current MSRV.
`Cargo.lock` is committed because examples are part of workspace validation.

Use the umbrella crate with explicit feature flags:

```toml
victors-lambdas = { version = "0.1", features = ["logger", "metrics"] }
```

Local examples use path dependencies until crates are published.

## Feature Docs

- [Logger](https://github.com/voctory/victors-lambdas/blob/main/docs/features/logger.md)
- [Lambda Metadata](https://github.com/voctory/victors-lambdas/blob/main/docs/features/metadata.md)
- [Metrics](https://github.com/voctory/victors-lambdas/blob/main/docs/features/metrics.md)
- [Tracer](https://github.com/voctory/victors-lambdas/blob/main/docs/features/tracer.md)
- [Parameters](https://github.com/voctory/victors-lambdas/blob/main/docs/features/parameters.md)
- [JMESPath](https://github.com/voctory/victors-lambdas/blob/main/docs/features/jmespath.md)
- [Data Masking](https://github.com/voctory/victors-lambdas/blob/main/docs/features/data-masking.md)
- [Kafka](https://github.com/voctory/victors-lambdas/blob/main/docs/features/kafka.md)
- [Streaming](https://github.com/voctory/victors-lambdas/blob/main/docs/features/streaming.md)
- [Parser](https://github.com/voctory/victors-lambdas/blob/main/docs/features/parser.md)
- [Batch](https://github.com/voctory/victors-lambdas/blob/main/docs/features/batch.md)
- [Idempotency](https://github.com/voctory/victors-lambdas/blob/main/docs/features/idempotency.md)
- [Validation](https://github.com/voctory/victors-lambdas/blob/main/docs/features/validation.md)
- [Feature Flags](https://github.com/voctory/victors-lambdas/blob/main/docs/features/feature-flags.md)
- [Event Handler](https://github.com/voctory/victors-lambdas/blob/main/docs/features/event-handler.md)
- [Testing](https://github.com/voctory/victors-lambdas/blob/main/docs/features/testing.md)

## Validation

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo check --workspace --all-targets --all-features
```
