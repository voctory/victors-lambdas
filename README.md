# Powertools Lambda Rust

Powertools Lambda Rust is an unofficial Rust toolkit for AWS Lambda functions, planned around utilities for structured
logging, custom metrics, tracing, parameter retrieval, event parsing, batch processing, validation, idempotency, and event
handling.

This repository is in its initial implementation phase. The workspace already contains the planned crate layout and
initial utility primitives; see [docs/porting-plan.md](docs/porting-plan.md) for the roadmap.

## Validation

The repository pins Rust `1.85.0`, the current MSRV for the initial Rust 2024 workspace.

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo check --workspace --all-targets --all-features
```
