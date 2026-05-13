# Powertools Lambda Rust Porting Plan

This document tracks implementation status for Powertools Lambda Rust, an unofficial Rust toolkit for AWS Lambda
functions. Keep public wording precise: describe it as unofficial and pre-release until project status changes.

## Current State

- Workspace: virtual Cargo workspace with resolver `3`, Rust 2024, Rust `1.85.0`, committed `Cargo.lock`, shared lints,
  a `release-lambda` profile, and CI for fmt, clippy, test, and check.
- Crates: one umbrella crate, `aws-lambda-powertools`, plus utility crates under `crates/`.
- Feature flags: the umbrella crate exposes `logger`, `metrics`, `tracer`, `parameters`, `parser`, `batch`,
  `idempotency`, `validation`, `event-handler`, and `all`.
- Example: `examples/basic-lambda` builds against the umbrella crate with all current utility features enabled.
- Publishing: no crates.io release is documented yet. Local examples use path dependencies.

## Goals

- Provide Rust-native utilities for common Lambda operational practices: structured logging, CloudWatch EMF metrics,
  tracing, parameter retrieval, event parsing, batch responses, validation, idempotency, and event handling.
- Keep provider integrations and heavier dependencies behind Cargo features so users do not pay for unused integrations.
- Preserve useful cross-language Powertools conventions where they fit Rust, especially environment variable names and
  utility boundaries.
- Design APIs around Rust traits, enums, builders, the type system, and error types instead of translating decorator or
  middleware patterns mechanically.

## Non-Goals for the Initial Phase

- Do not present this repository as official.
- Do not add Lambda layers initially. Rust users normally ship compiled binaries; revisit extension or layer packaging
  only when there is a concrete use case.
- Do not port every Python-only utility before the core Rust API foundations are stable.
- Do not copy proprietary code, names, comments, business logic, or implementation details from adjacent private
  repositories.

## Landed First Tranche

| Area | Implemented locally | Still missing |
| --- | --- | --- |
| Workspace | Workspace layout, shared package metadata, lints, lockfile, Rust toolchain, CI, `release-lambda` profile | Release automation, changelog, publishing workflow |
| Umbrella crate | Feature-gated re-exports and a prelude across current utility crates | Published crate metadata review and docs.rs examples |
| Core | `ServiceConfig`, env constants and parsers, cold-start tracking, user-agent metadata | Cross-crate error conventions beyond concrete utility errors |
| Logger | `LoggerConfig`, `LogLevel`, `Logger`, `LogEntry`, `LogValue`, `LambdaContextFields`, JSON rendering, persistent fields, temporary fields, event rendering toggle, level filtering, debug sampling, correlation ID helpers, Lambda context fields, key redaction, stdout emission | Custom formatter/redaction hook APIs, `tracing` subscriber integration |
| Metrics | `MetricsConfig`, `Metric`, `MetricUnit`, `MetricResolution`, `MetadataValue`, EMF JSON renderer, request dimensions, default dimensions, metadata, name/value validation, service dimension, cold-start metric, high-resolution metric definitions, stdout flush API, CloudWatch limits | Async handler ergonomics, overflow auto-flush convenience, timestamp customization |
| Tracer | `TracerConfig`, `Tracer`, `TraceContext`, capture flags, injectable env sources, X-Ray header parsing, `TraceSegment`, `TraceValue` | Real `tracing` spans, OpenTelemetry, X-Ray propagation/export |
| Parameters | `ParameterProvider`, `Parameters`, `Parameter`, `CachePolicy`, in-memory provider | SSM, Secrets Manager, AppConfig, DynamoDB providers, decrypt options, forced fetch, transforms |
| Parser | `EventParser`, `ParsedEvent`, `ParseError`, serde JSON string/slice/value parsing | `aws_lambda_events` envelopes, Powertools adapters, shared event fixtures, schema-aware parsing |
| Batch | `BatchRecord`, `BatchProcessor`, `BatchProcessingReport`, `BatchRecordResult`, `BatchItemFailure`, `BatchResponse` | SQS/Kinesis/DynamoDB source adapters, FIFO early-stop behavior, concurrent processing |
| Validation | `Validator`, `Validate`, `ValidationError`, required text, length, range, and custom predicate helpers | JSON Schema backend, schema cache, inbound/outbound validation wrappers |
| Idempotency | `IdempotencyConfig`, `IdempotencyKey`, `IdempotencyStatus`, `IdempotencyRecord`, store trait/error/result, in-memory store | Handler wrapper, key extraction, payload hashing, result replay, DynamoDB store, concurrency semantics |
| Event handler | `Method`, method parsing/matching, `Request`, `Response`, `PathParams`, `Route`, `Router`, static/dynamic path precedence, `ANY` routes, and 404 dispatch | API Gateway/event adapters, async handlers, middleware, CORS, compression, AppSync, Bedrock Agent |
| Testing | `LambdaContextStub` and parameter provider stub re-export | Fixture loaders, fake AWS providers, handler harnesses |

## Next Durable Work

The next durable work should turn the landed primitives into Lambda-facing utilities:

1. Harden logger and metrics: logger custom formatter/redaction hook APIs, async metrics handler ergonomics, and
   buildable docs/snippets.
2. Replace tracer records with real `tracing` span integration, then add optional OpenTelemetry and X-Ray-compatible
   propagation/export features.
3. Add parameter provider integrations behind feature flags. Confirm the AWS SDK MSRV impact before enabling those
   dependencies.
4. Add parser envelopes and fixtures using `aws_lambda_events` as the default event model source.
5. Expand batch and idempotency together where AWS retry semantics overlap: source-specific batch adapters, key
   extraction, payload hashing, DynamoDB persistence, and replay behavior.
6. Add event adapters for HTTP routing after parser/event model choices are stable.

## Crate Strategy

| Crate | Current role | Notes |
| --- | --- | --- |
| `aws-lambda-powertools` | Primary user-facing crate | Depends on support crates through optional dependencies and re-exports enabled utilities |
| `aws-lambda-powertools-core` | Shared foundations | Keep small: config, env, cold start, metadata, and other genuine cross-crate foundations |
| `aws-lambda-powertools-logger` | Structured logs | JSON renderer, sampling, correlation IDs, Lambda context fields, and key redaction exist; next work should avoid forcing one subscriber setup |
| `aws-lambda-powertools-metrics` | CloudWatch EMF metrics | Renderer, flush API, high-resolution metrics, and default dimensions exist; next work is async handler ergonomics and feature completeness |
| `aws-lambda-powertools-tracer` | Tracing facade | Segment records exist; next work is integration with Rust tracing/export pipelines |
| `aws-lambda-powertools-parameters` | Parameter retrieval | Trait, cache facade, and in-memory provider exist; AWS providers are next |
| `aws-lambda-powertools-parser` | Event parsing | serde JSON facade exists; event envelopes are next |
| `aws-lambda-powertools-batch` | Partial batch responses | Generic sequential processing exists; source-specific behavior is next |
| `aws-lambda-powertools-idempotency` | Deduplication | Records and stores exist; handler semantics and providers are next |
| `aws-lambda-powertools-validation` | Payload validation | Basic validators exist; JSON Schema remains optional future work |
| `aws-lambda-powertools-event-handler` | Routing | Dependency-free routing exists; next work is event adapters and middleware |
| `aws-lambda-powertools-testing` | Test helpers | Minimal stubs exist; expand only as real utilities need them |

Provider features should live on the owning utility crate first and be re-exposed by the umbrella crate only when that is
ergonomic for users.

## Feature Flags

Implemented umbrella features:

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

Likely future provider and integration features:

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

## Environment Variable Compatibility

These names are reserved in the core crate. Some are already read by landed utilities; others are compatibility anchors
for future integrations.

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

Rust APIs should prefer typed configuration builders, but keeping these names stable helps deployments that already use
Powertools conventions.

## Planned Additions

- `docs/getting-started/installation.md`
- `docs/features/<feature>.md` as each utility becomes usable
- buildable snippets under `examples/snippets/<feature>/`
- shared JSON event fixtures under `tests/events/`
- gated end-to-end tests under `tests/e2e/`
- one deployable example app after logger and metrics are usable in realistic handlers
- root `CHANGELOG.md` before publishing

## Backlog

- [x] Establish the virtual workspace, crate layout, MSRV, lockfile, lints, and CI.
- [x] Add first-pass core config, environment parsing, cold-start, and metadata helpers.
- [x] Add first-pass structured JSON logger.
- [x] Add first-pass CloudWatch EMF metrics renderer.
- [x] Add first-pass parser, batch, validation, parameters, idempotency, tracer, event-handler, and testing surfaces.
- [x] Complete tracer records, HTTP method/request/response work, prelude exports, and the expanded workspace example.
- [ ] Add user-facing docs and snippets for implemented logger and metrics behavior.
- [x] Add logger sampling, key redaction, correlation IDs, and Lambda context helpers.
- [ ] Add logger custom formatter/redaction hook APIs and `tracing` subscriber integration.
- [x] Add metrics flush ergonomics, high-resolution metrics, and default dimension helpers.
- [ ] Implement `tracing` span integration and optional OpenTelemetry/X-Ray features.
- [ ] Implement AWS-backed parameter providers behind feature flags.
- [ ] Add event envelopes and fixtures based on `aws_lambda_events`.
- [ ] Add source-specific batch processors and retry semantics.
- [ ] Add JSON Schema validation behind an optional feature.
- [ ] Add idempotency handler workflow and DynamoDB persistence.
- [ ] Add API Gateway/event adapters, middleware, CORS, and related HTTP routing integrations.
- [ ] Add release notes, crates.io publishing checks, docs.rs coverage, and provenance/SBOM work after API boundaries
  settle.

## Validation Policy

Canonical local validation:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo check --workspace --all-targets --all-features
```

CI uses the same checks with `--locked` where dependency resolution matters.

## Resolved Initial Decisions

- Project status: remain unofficial for now.
- MSRV: start at Rust `1.85.0`, the Rust 2024 floor. Raise it only for required dependencies or language features, and
  document MSRV bumps as release-visible changes.
- Publishing: keep one synchronized workspace version. If the current multi-crate graph is published, publish support
  crates before the umbrella crate.
- Event types: use `aws_lambda_events` by default and own only Powertools-specific adapters, envelopes, fixtures, and
  missing models.
- Tracing: build on Rust `tracing` spans first, then add optional OpenTelemetry and X-Ray propagation/export integration.
- Contributor commands: keep plain Cargo commands as the canonical workflow. Add `just` or `make` only later as optional
  convenience wrappers.
- Lockfile: keep `Cargo.lock` committed for reproducible workspace and example validation.
