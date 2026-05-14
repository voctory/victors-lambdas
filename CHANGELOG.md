# Changelog

All notable changes to Victor's Lambdas will be documented in this file.

This project follows semantic versioning after `1.0.0`. Until then, `0.x`
releases may include breaking API changes.

## 0.1.0 - Unreleased

Initial pre-release of Victor's Lambdas, an unofficial side project for Rust
Lambda utilities.

### Added

- Umbrella crate `victors-lambdas` with feature-gated re-exports.
- Core foundations for service configuration, environment parsing,
  cold-start tracking, and user-agent metadata.
- Utilities for Lambda metadata, structured logging, CloudWatch EMF metrics,
  tracing, parameters, JMESPath, data masking, Kafka records, streaming,
  parser/event models, batch processing, idempotency, validation, feature
  flags, event handling, and testing helpers.
- Buildable examples and feature-specific snippets under `examples/`.
- Pre-publish validation and release checklist in `docs/release.md`.

### Notes

- This crate family is not affiliated with, endorsed by, sponsored by, or owned
  by Amazon Web Services, AWS, or Amazon.com, Inc.
- APIs are pre-release and should be treated as unstable until the project has
  real downstream adoption feedback.
