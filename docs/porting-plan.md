# Powertools Lambda Rust Porting Plan

This document tracks implementation status for Powertools Lambda Rust, an unofficial Rust toolkit for AWS Lambda
functions. Keep public wording precise: describe it as unofficial and pre-release until project status changes.

## Current State

- Workspace: virtual Cargo workspace with resolver `3`, Rust 2024, Rust `1.85.0`, committed `Cargo.lock`, shared lints,
  a `release-lambda` profile, and CI for fmt, clippy, test, and check.
- Crates: one umbrella crate, `aws-lambda-powertools`, plus utility crates under `crates/`.
- Feature flags: the umbrella crate exposes `logger`, `logger-tracing`, `metrics`, `tracer`, `parameters`,
  `parameters-appconfig`, `parameters-dynamodb`, `parameters-secrets`, `parameters-ssm`, `parser`,
  `parser-aws-lambda-events`, `batch`, `batch-aws-lambda-events`, `idempotency`, `idempotency-dynamodb`,
  `validation`, `validation-jsonschema`, `event-handler`, `event-handler-compression`,
  `event-handler-aws-lambda-events`, and `all`.
- Examples: `examples/basic-lambda` builds against the umbrella crate with all current utility features enabled, and
  `examples/snippets/logger` plus `examples/snippets/metrics` provide buildable docs snippets.
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
| Logger | `LoggerConfig`, `LogLevel`, `Logger`, `LogEntry`, `LogValue`, `LogFormatter`, `LogRedactor`, `JsonLogFormatter`, `LambdaContextFields`, `LoggerLayer`, JSON rendering, persistent fields, temporary fields, event rendering toggle, level filtering, debug sampling, correlation ID helpers, Lambda context fields, key redaction, custom formatter/redaction hook APIs, optional `tracing` subscriber integration, stdout emission, and initial docs/snippet | Broader handler examples |
| Metrics | `MetricsConfig`, `Metric`, `MetricUnit`, `MetricResolution`, `MetadataValue`, EMF JSON renderer, request dimensions, default dimensions, metadata, name/value validation, service dimension, cold-start metric, high-resolution metric definitions, stdout flush API, explicit timestamp rendering/writing, opt-in overflow flush helpers, async capture helpers, CloudWatch limits, and initial docs/snippet | Broader handler examples |
| Tracer | `TracerConfig`, `Tracer`, `TraceContext`, capture flags, injectable env sources, X-Ray header parsing, `TraceSegment`, `TraceValue` | Real `tracing` spans, OpenTelemetry, X-Ray propagation/export |
| Parameters | `ParameterProvider`, `AsyncParameterProvider`, `Parameters`, `AsyncParameters`, `Parameter`, `CachePolicy`, async provider/retrieval errors, in-memory provider, optional SSM single-parameter, by-name, and path providers with decryption plus set operations, optional Secrets Manager, AppConfig, and DynamoDB providers, force-fetch support, JSON transforms, and base64 binary transforms | Broader provider docs/examples |
| Parser | `EventParser`, `ParsedEvent`, `ParseError`, serde JSON string/slice/value parsing, optional `aws_lambda_events` API Gateway REST/HTTP/WebSocket API body, AppSync direct resolver arguments/source, Bedrock Agent input text, ALB target group body, Lambda Function URL body, VPC Lattice v1/v2 body, EventBridge detail, SQS body, SNS message, CloudWatch Logs message, Kinesis record data, Firehose record data, DynamoDB stream image, and Kafka record value envelopes | Broader `aws_lambda_events` envelopes, Powertools adapters, shared event fixtures, schema-aware parsing |
| Batch | `BatchRecord`, `BatchProcessor`, `BatchProcessingReport`, `BatchRecordResult`, `BatchItemFailure`, `BatchResponse`, sequential and concurrent generic processing, stream checkpoint helpers, optional `aws_lambda_events` SQS, Kinesis, and DynamoDB stream adapters, SQS FIFO early-stop behavior | Parser-integrated processors and larger examples |
| Validation | `Validator`, `Validate`, `ValidationError`, required text, length, range, custom predicate helpers, inbound/outbound validation wrappers, optional local JSON Schema backend, and compiled schema cache | Handler middleware/docs integration |
| Idempotency | `IdempotencyConfig`, `IdempotencyKey`, `Idempotency`, `AsyncIdempotency`, `IdempotencyOutcome`, typed workflow errors, SHA-256 JSON payload hashing, JSON Pointer key extraction, sync and async handler wrappers, payload hash validation, result replay, sync and async store traits/errors/results, in-memory store, and optional DynamoDB store | Lambda-context timeout integration and richer examples |
| Event handler | `Method`, method parsing/matching, `Request`, `Response`, `PathParams`, `Route`, `AsyncRoute`, `Router`, `AsyncRouter`, static/dynamic path precedence, `ANY` routes, 404 dispatch, request/response middleware, `CorsConfig`, preflight responses, routed/404 CORS headers, optional gzip/deflate compression middleware, optional AppSync direct resolver routing, optional Bedrock Agent adapter, optional ALB, Lambda Function URL, and VPC Lattice adapters, and optional API Gateway REST API v1 / HTTP API v2 / WebSocket API adapters | Additional resolver families and docs |
| Testing | `LambdaContextStub`, parameter provider stub re-export, text/bytes fixture readers, and JSON fixture decoder | Fake AWS providers, handler harnesses |

## Next Durable Work

The next durable work should turn the landed primitives into Lambda-facing utilities:

1. Replace tracer records with real `tracing` span integration, then add optional OpenTelemetry and X-Ray-compatible
   propagation/export features.
2. Expand parameter provider docs and examples. Keep AWS SDK dependencies aligned with the documented MSRV.
3. Expand parser envelopes and fixtures using `aws_lambda_events` as the default event model source.
4. Expand idempotency where AWS retry semantics overlap: Lambda-context timeout handling and richer examples.
5. Add event-handler adapters for additional resolver families and document the current HTTP, WebSocket, ALB, Lambda
   Function URL, VPC Lattice, AppSync, and Bedrock surfaces.

## Crate Strategy

| Crate | Current role | Notes |
| --- | --- | --- |
| `aws-lambda-powertools` | Primary user-facing crate | Depends on support crates through optional dependencies and re-exports enabled utilities |
| `aws-lambda-powertools-core` | Shared foundations | Keep small: config, env, cold start, metadata, and other genuine cross-crate foundations |
| `aws-lambda-powertools-logger` | Structured logs | JSON renderer, sampling, correlation IDs, Lambda context fields, key redaction, custom formatter/redaction hooks, optional `tracing` subscriber layer, and initial docs/snippet exist; next work is broader handler examples |
| `aws-lambda-powertools-metrics` | CloudWatch EMF metrics | Renderer, flush API, high-resolution metrics, default dimensions, explicit timestamps, overflow flush helpers, async capture helpers, and initial docs/snippet exist; next work is broader handler examples |
| `aws-lambda-powertools-tracer` | Tracing facade | Segment records exist; next work is integration with Rust tracing/export pipelines |
| `aws-lambda-powertools-parameters` | Parameter retrieval | Sync and async traits, cache facades, async provider/retrieval errors, in-memory provider, optional SSM single-parameter, by-name, and path providers plus set operations, optional Secrets Manager, AppConfig, and DynamoDB providers, force-fetch support, and JSON/base64 transforms exist; broader examples are next |
| `aws-lambda-powertools-parser` | Event parsing | serde JSON facade plus API Gateway REST API, HTTP API, and WebSocket API, AppSync direct resolver arguments/source, Bedrock Agent input text, ALB, Lambda Function URL, VPC Lattice, SQS, SNS, EventBridge, CloudWatch Logs, Kinesis, Firehose, DynamoDB stream image, and Kafka `aws_lambda_events` envelopes exist; broader envelope coverage and fixtures are next |
| `aws-lambda-powertools-batch` | Partial batch responses | Generic sequential/concurrent processing, stream checkpoint helpers, and SQS, Kinesis, and DynamoDB stream adapters exist; parser-integrated processors and examples are next |
| `aws-lambda-powertools-idempotency` | Deduplication | JSON payload hashing, key extraction, sync and async handler workflows, replay, records, in-memory store, and optional DynamoDB persistence exist; Lambda-context timeout integration and richer examples are next |
| `aws-lambda-powertools-validation` | Payload validation | Basic validators, inbound/outbound wrappers, optional JSON Schema validation, and schema caching exist; next work is handler middleware and examples |
| `aws-lambda-powertools-event-handler` | Routing | Dependency-free sync/async routing, middleware, CORS, optional compression middleware, optional AppSync direct resolver routing, optional Bedrock Agent adapter, optional ALB, Lambda Function URL, and VPC Lattice adapters, and optional API Gateway REST API, HTTP API, and WebSocket API adapters exist; next work is additional event adapters and docs |
| `aws-lambda-powertools-testing` | Test helpers | Context stubs, parameter provider stubs, and fixture loaders exist; expand fake providers and handler harnesses only as real utilities need them |

Provider features should live on the owning utility crate first and be re-exposed by the umbrella crate only when that is
ergonomic for users.

## Feature Flags

Implemented umbrella features:

- `logger`
- `logger-tracing`
- `metrics`
- `tracer`
- `parameters`
- `parameters-appconfig`
- `parameters-dynamodb`
- `parameters-secrets`
- `parameters-ssm`
- `parser`
- `parser-aws-lambda-events`
- `batch`
- `batch-aws-lambda-events`
- `idempotency`
- `idempotency-dynamodb`
- `validation`
- `validation-jsonschema`
- `event-handler`
- `event-handler-compression`
- `event-handler-aws-lambda-events`
- `all`

Likely future provider and integration features:

- `idempotency-redis`
- `parser-serde`
- `parser-schemars`
- `tracer-otel`
- `tracer-xray-propagation`

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
- [x] Add user-facing docs and snippets for implemented logger and metrics behavior.
- [x] Add logger sampling, key redaction, correlation IDs, and Lambda context helpers.
- [x] Add logger custom formatter/redaction hook APIs.
- [x] Add logger `tracing` subscriber integration.
- [x] Add metrics flush ergonomics, high-resolution metrics, and default dimension helpers.
- [x] Add metrics explicit timestamp rendering and overflow flush helpers.
- [x] Add metrics async capture helpers.
- [ ] Implement `tracing` span integration and optional OpenTelemetry/X-Ray features.
- [x] Add parameter force-fetch and local value transforms.
- [x] Add async parameter provider facade and errors for AWS SDK-backed providers.
- [x] Add SSM single-parameter provider behind a feature flag.
- [x] Add SSM by-name and path retrieval helpers behind a feature flag.
- [x] Add SSM set-parameter helper behind a feature flag.
- [x] Add Secrets Manager provider behind a feature flag.
- [x] Add AppConfig provider behind a feature flag.
- [x] Add DynamoDB parameter provider behind a feature flag.
- [x] Implement remaining AWS-backed parameter providers behind feature flags.
- [x] Add initial SQS, SNS, and EventBridge parser envelopes based on `aws_lambda_events`.
- [x] Add API Gateway REST API and HTTP API parser body envelopes based on `aws_lambda_events`.
- [x] Add API Gateway WebSocket API parser body envelope based on `aws_lambda_events`.
- [x] Add AppSync direct resolver parser argument/source envelopes based on `aws_lambda_events`.
- [x] Add Bedrock Agent input text parser envelope based on `aws_lambda_events`.
- [x] Add ALB target group parser body envelope based on `aws_lambda_events`.
- [x] Add Lambda Function URL parser body envelope based on `aws_lambda_events`.
- [x] Add VPC Lattice v1/v2 parser body envelopes based on `aws_lambda_events`.
- [x] Add Kinesis and Firehose parser record-data envelopes based on `aws_lambda_events`.
- [x] Add DynamoDB stream parser image envelopes based on `aws_lambda_events`.
- [x] Add CloudWatch Logs parser message envelope based on `aws_lambda_events`.
- [x] Add Kafka parser record-value envelopes based on `aws_lambda_events`.
- [ ] Expand parser envelopes and fixtures based on `aws_lambda_events`.
- [x] Add SQS source-specific batch processing and FIFO retry semantics.
- [x] Add Kinesis and DynamoDB stream batch processors and retry semantics.
- [x] Add generic concurrent batch processing.
- [x] Add stream checkpoint helpers for Kinesis and DynamoDB retry semantics.
- [x] Add JSON Schema validation behind an optional feature.
- [x] Add validation schema cache and inbound/outbound wrappers.
- [x] Add idempotency handler workflow, key hashing, payload validation, and replay behavior.
- [x] Add async idempotency store and handler workflow.
- [x] Add DynamoDB idempotency persistence and provider-level concurrency semantics.
- [x] Add API Gateway REST API and HTTP API adapters for event-handler routing.
- [x] Add event-handler CORS configuration and preflight handling.
- [x] Add event-handler request/response middleware.
- [x] Add event-handler async handlers and related HTTP routing integrations.
- [x] Add event-handler gzip/deflate response compression middleware.
- [x] Add event-handler AppSync direct resolver routing.
- [x] Add event-handler Bedrock Agent adapter.
- [x] Add event-handler ALB adapter.
- [x] Add event-handler Lambda Function URL adapter.
- [x] Add event-handler VPC Lattice v1/v2 adapters.
- [x] Add event-handler API Gateway WebSocket adapter.
- [x] Add testing fixture loaders.
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
