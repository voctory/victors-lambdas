# Powertools Lambda Rust Porting Plan

This document tracks the initial architecture and implementation backlog for Powertools Lambda Rust. The project is an
unofficial Rust toolkit for AWS Lambda functions and should avoid wording that implies official AWS ownership.

## Goals

- Provide Rust-native utilities for common AWS Lambda operational practices: structured logging, CloudWatch EMF metrics,
  tracing, parameter retrieval, event parsing, batch responses, validation, idempotency, and event handling.
- Keep Lambda binaries small by making provider integrations and heavier dependencies opt-in Cargo features.
- Preserve useful cross-language Powertools conventions where they make sense in Rust, especially environment variable
  names and conceptual utility boundaries.
- Build public APIs around Rust traits, enums, builders, ownership, and error types rather than mechanically translating
  Python decorators or TypeScript middleware patterns.

## Non-Goals for the Initial Phase

- Do not claim official AWS-owned project status.
- Remain clearly unofficial for now. Revisit project status only if there is an explicit future governance decision.
- Do not add Lambda layers initially. Rust users normally ship compiled binaries; extension or layer packaging can be
  revisited later if a concrete use case appears.
- Do not port every Python-only utility before core Rust API foundations are stable.
- Do not copy proprietary code or implementation details from adjacent private repositories.

## Target Repository Layout

```text
.
|-- Cargo.toml
|-- Cargo.lock
|-- AGENTS.md
|-- README.md
|-- crates/
|   |-- aws-lambda-powertools/              # umbrella crate and feature-gated re-exports
|   |-- aws-lambda-powertools-core/         # env, config, cold start, metadata, user-agent
|   |-- aws-lambda-powertools-logger/       # structured logging
|   |-- aws-lambda-powertools-metrics/      # CloudWatch EMF metrics
|   |-- aws-lambda-powertools-tracer/       # tracing/X-Ray facade
|   |-- aws-lambda-powertools-parameters/   # SSM, Secrets Manager, AppConfig, DynamoDB providers
|   |-- aws-lambda-powertools-parser/       # event envelopes and typed parsing
|   |-- aws-lambda-powertools-batch/        # partial batch responses
|   |-- aws-lambda-powertools-idempotency/  # idempotent handler support
|   |-- aws-lambda-powertools-validation/   # schema validation
|   |-- aws-lambda-powertools-event-handler/# HTTP/AppSync/Bedrock routing
|   `-- aws-lambda-powertools-testing/      # test fixtures and handler test helpers
|-- docs/
|   |-- porting-plan.md
|   |-- getting-started/
|   |-- features/
|   `-- contributing/
|-- examples/
|   |-- basic-lambda/
|   `-- snippets/
`-- tests/
    |-- events/
    `-- e2e/
```

The current repository has the first workspace skeleton in place. The `docs/`, `examples/snippets/`, and `tests/`
subtrees should grow as real utility implementations land.

## Crate Strategy

| Crate | Purpose | Initial Public Surface |
| --- | --- | --- |
| `aws-lambda-powertools` | Primary user-facing crate | Feature-gated modules and a small prelude |
| `aws-lambda-powertools-core` | Shared foundations | `ServiceConfig`, environment names, cold start tracking, metadata |
| `aws-lambda-powertools-logger` | Structured logs | `Logger`, `LoggerConfig`, `LogLevel` |
| `aws-lambda-powertools-metrics` | EMF metrics | `Metrics`, `MetricsConfig`, `Metric`, `MetricUnit` |
| `aws-lambda-powertools-tracer` | Tracing facade | `Tracer`, `TracerConfig`, `TraceContext` |
| `aws-lambda-powertools-parameters` | Parameter retrieval | `ParameterProvider`, provider clients, cache policy |
| `aws-lambda-powertools-parser` | Event parsing | event envelope traits, AWS event types, serde integration |
| `aws-lambda-powertools-batch` | Partial batch responses | record processors and failure response builders |
| `aws-lambda-powertools-idempotency` | Deduplication | key extraction, persistence traits, record state machine |
| `aws-lambda-powertools-validation` | Payload validation | schema validators, validation errors |
| `aws-lambda-powertools-event-handler` | Routing | routers, route params, middleware, response types |
| `aws-lambda-powertools-testing` | Test helpers | context stubs, fixture loaders, fake providers |

The umbrella crate should depend on feature crates through optional dependencies. The intended user path is:

```toml
aws-lambda-powertools = { version = "0.1", features = ["logger", "metrics"] }
```

When the workspace is ready for crates.io, publish the support crates as implementation crates before publishing the
umbrella crate. `aws-lambda-powertools-core` should be published only as a support crate for the workspace graph; normal
users should depend on `aws-lambda-powertools`.

## Feature Flags

Initial umbrella features:

- `logger`
- `metrics`
- `tracer`
- `parameters`
- `parser`
- `batch`
- `idempotency`
- `validation`
- `event-handler`
- `all`

Likely provider and integration features:

- `parameters-ssm`
- `parameters-secrets`
- `parameters-appconfig`
- `parameters-dynamodb`
- `idempotency-dynamodb`
- `idempotency-redis`
- `validation-jsonschema`
- `parser-serde`
- `events-aws-lambda-events`
- `parser-schemars`
- `tracer-otel`
- `tracer-xray-propagation`
- `event-handler-http`
- `event-handler-appsync`
- `event-handler-bedrock-agent`

Provider features should live on the owning feature crate first and be re-exposed by the umbrella crate only when that is
ergonomic for users.

## Environment Variable Compatibility

Keep these names stable unless there is a documented Rust-specific reason to diverge:

- `POWERTOOLS_SERVICE_NAME`
- `POWERTOOLS_LOG_LEVEL`
- `POWERTOOLS_LOGGER_LOG_EVENT`
- `POWERTOOLS_LOGGER_SAMPLE_RATE`
- `POWERTOOLS_METRICS_NAMESPACE`
- `POWERTOOLS_METRICS_DISABLED`
- `POWERTOOLS_METRICS_FUNCTION_NAME`
- `POWERTOOLS_TRACE_ENABLED`
- `POWERTOOLS_TRACER_CAPTURE_RESPONSE`
- `POWERTOOLS_TRACER_CAPTURE_ERROR`
- `POWERTOOLS_PARAMETERS_MAX_AGE`
- `POWERTOOLS_PARAMETERS_SSM_DECRYPT`
- `POWERTOOLS_IDEMPOTENCY_DISABLED`
- `POWERTOOLS_DEV`
- `POWERTOOLS_DEBUG`

Rust should prefer strongly typed config builders over requiring environment variables, but the environment names should
remain compatible for Lambda deployments that already use Powertools conventions.

## Implementation Phases

### Phase 0: Workspace Foundation

- Keep the workspace compiling with `cargo check --workspace --all-targets --all-features`.
- Keep clippy clean with `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- Maintain CI for fmt, clippy, test, check, and locked builds.
- Add issue/PR templates and release automation once the first APIs settle.
- Use Rust `1.85.0` as the initial MSRV because it is the Rust 2024 edition floor. Raise MSRV only when a required
  dependency or language feature justifies it. Current AWS SDK provider crates may force a higher MSRV when provider
  features land.

### Phase 1: Core

- Expand environment helpers for booleans, numbers, durations, and known Powertools variables.
- Add shared error types only when there is a real cross-crate contract.
- Add cold-start tracking with deterministic tests.
- Add user-agent metadata for AWS SDK client configuration.
- Define conventions for builder types, error names, and feature gates.

### Phase 2: Logger

- Implement structured JSON logging.
- Support service name, log level, sampling, persistent keys, temporary keys, and event logging.
- Support correlation ID extraction paths where practical.
- Integrate with the Rust `tracing` ecosystem without forcing users into one subscriber setup.
- Add redaction hooks before logging full events.
- Test serialization, level filtering, sampling, context reset, and Lambda invocation reuse behavior.

### Phase 3: Metrics

- Implement CloudWatch EMF serialization.
- Support namespace, service dimension, default dimensions, high-resolution metrics, metadata, and cold-start metrics.
- Add a flush API that is explicit and ergonomic in async Lambda handlers.
- Prevent invalid metric names, dimensions, and unit combinations through validation.
- Add golden JSON tests for EMF payloads.

### Phase 4: Tracer

- Build the public tracer API as a thin Powertools facade over Rust `tracing` spans. Add OpenTelemetry integration behind
  `tracer-otel`, with X-Ray compatible propagation/export behavior behind `tracer-xray-propagation`.
- Support capture response, capture error, service annotations, and subsegments.
- Avoid naming conflicts with the `tracing` crate by keeping the public module name `tracer`.
- Add tests for disabled tracing and error capture behavior.

### Phase 5: Parameters

- Define `ParameterProvider` and cache traits.
- Implement SSM Parameter Store, Secrets Manager, AppConfig, and DynamoDB providers behind feature flags.
- Support max age, forced fetch, transform/decode hooks, and secure string decrypt options.
- Add fake providers in the testing crate.
- Add integration tests gated by explicit AWS environment variables.

### Phase 6: Parser and Event Types

- Use `aws_lambda_events` as the default source for AWS event structs. Own only Powertools-specific envelopes, adapters,
  extension traits, fixtures, and any missing event models that cannot reasonably be represented upstream.
- Add event envelopes for API Gateway REST/HTTP/WebSocket, ALB, EventBridge, SQS, SNS, DynamoDB Streams, Kinesis,
  Kinesis Firehose, S3, CloudWatch Logs, AppSync, Kafka, and Bedrock Agent events.
- Use serde-based parsing with clear error messages and optional schema support.
- Add shared JSON fixtures under `tests/events/`.

### Phase 7: Batch

- Support SQS, Kinesis, and DynamoDB Streams partial batch responses.
- Model record success/failure explicitly.
- Provide sequential and concurrent processing strategies.
- Support FIFO behavior and early-stop rules where required by AWS semantics.
- Add tests for partial failure response shape and retry behavior.

### Phase 8: Validation

- Implement JSON Schema validation behind an optional feature.
- Support inbound and outbound validation helpers.
- Keep schema compilation cache explicit and testable.
- Provide errors that include path, schema location, and validation keyword where supported by the backend library.

### Phase 9: Idempotency

- Model idempotency records with an enum state machine.
- Define persistence traits and implement DynamoDB first.
- Support key extraction, payload hashing, in-progress expiry, result caching, and local in-memory cache.
- Add tests for first execution, duplicate in-progress execution, replayed completed execution, and expired records.

### Phase 10: Event Handler

- Start with API Gateway HTTP routing.
- Add route params, query params, headers, cookies, binary responses, CORS, compression hooks, middleware, and typed
  responses.
- Add AppSync, AppSync Events, and Bedrock Agent handlers after HTTP stabilizes.
- Preserve route precedence with tests, especially static routes before dynamic peers.

### Phase 11: Docs and Examples

- Add `docs/getting-started/installation.md`.
- Add `docs/features/<feature>.md` for each utility as it becomes usable.
- Add buildable snippets under `examples/snippets/<feature>/`.
- Add one deployable example app once logger and metrics are functional.
- Publish API docs through docs.rs and keep public examples as doctests where possible.

### Phase 12: Release and Distribution

- Keep one synchronized workspace version until there is a reason to version crates independently.
- Use a root `CHANGELOG.md` and `vX.Y.Z` tags.
- Add release drafter or equivalent conventional-title changelog automation.
- Add crates.io publishing only after API naming and feature boundaries settle.
- Add provenance/SBOM work after the first pre-release.

## Testing Plan

- Unit tests: colocated in modules for small pure behavior.
- Integration tests: crate-level `tests/` directories for public API behavior.
- Shared fixtures: root `tests/events/` for JSON events reused by parser, batch, and event handler tests.
- End-to-end tests: root `tests/e2e/`, gated behind explicit AWS account and region variables.
- Performance tests: add only for hot paths such as JSON logging, EMF generation, parser envelopes, and idempotency key
  hashing.

## Validation Policy

Canonical local validation:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo check --workspace --all-targets --all-features
```

CI should use `--locked` once dependency versions are introduced beyond path-only workspace crates.

## Resolved Initial Decisions

- Project status: remain unofficial for now.
- MSRV: start at Rust `1.85.0`, the Rust 2024 floor. Raise it only for required dependencies or language features, and
  document MSRV bumps as release-visible changes.
- Publishing: keep one synchronized workspace version. Publish `aws-lambda-powertools-core` as a support crate if the
  current multi-crate graph is published; otherwise the umbrella crate cannot depend on it from crates.io. Do not present
  core as the normal user entry point.
- Event types: use `aws_lambda_events` by default and own only Powertools-specific adapters, envelopes, fixtures, and
  missing models.
- Tracing: build on Rust `tracing` spans first, then add optional OpenTelemetry and X-Ray propagation/export integration.
- Contributor commands: keep plain Cargo commands as the canonical workflow. Add `just` or `make` only later as optional
  convenience wrappers.
- Lockfile: keep `Cargo.lock` committed for reproducible workspace and example validation.
