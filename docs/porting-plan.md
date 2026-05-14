# Powertools Lambda Rust Porting Plan

This document tracks implementation status for Powertools Lambda Rust, an unofficial Rust toolkit for AWS Lambda
functions. Keep public wording precise: describe it as unofficial and pre-release until project status changes.

## Current State

- Workspace: virtual Cargo workspace with resolver `3`, Rust 2024, Rust `1.85.0`, committed `Cargo.lock`, shared lints,
  a `release-lambda` profile, and CI for fmt, clippy, test, and check.
- Crates: one umbrella crate, `aws-lambda-powertools`, plus utility crates under `crates/`.
- Feature flags: the umbrella crate exposes `logger`, `logger-tracing`, `metrics`, `tracer`, `tracer-opentelemetry`,
  `tracer-tracing`, `parameters`, `parameters-appconfig`, `parameters-dynamodb`, `parameters-secrets`, `parameters-ssm`,
  `parser`, `parser-aws-lambda-events`, `batch`, `batch-aws-lambda-events`, `batch-parser`, `idempotency`,
  `idempotency-dynamodb`, `feature-flags`, `feature-flags-appconfig`, `validation`, `validation-jsonschema`,
  `event-handler`, `event-handler-appsync-events`, `event-handler-bedrock-agent-functions`,
  `event-handler-compression`, `event-handler-validation`, `event-handler-aws-lambda-events`, and `all`.
- Examples: `examples/basic-lambda` builds against the umbrella crate with all current utility features enabled, and
  feature-specific crates under `examples/snippets/` provide buildable docs snippets.
- Publishing: no crates.io release is documented yet. Local examples use path dependencies.

## Goals

- Provide Rust-native utilities for common Lambda operational practices: structured logging, CloudWatch EMF metrics,
  tracing, parameter retrieval, event parsing, batch responses, validation, idempotency, feature flags, and event
  handling.
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
| Tracer | `TracerConfig`, `Tracer`, `TraceContext`, capture flags, injectable env sources, X-Ray header parsing/rendering, `TraceSegment`, `TraceValue`, optional X-Ray-compatible subsegment document rendering, optional X-Ray daemon UDP transport, optional `tracing` span integration, optional OpenTelemetry span builder/attribute export, buildable OpenTelemetry SDK/stdout exporter snippet, and initial docs/snippet | Production OpenTelemetry exporter examples |
| Parameters | `ParameterProvider`, `AsyncParameterProvider`, `Parameters`, `AsyncParameters`, `Parameter`, `ParameterTransform`, `ParameterValue`, `CachePolicy`, async provider/retrieval errors, in-memory provider, optional SSM single-parameter, by-name, and path providers with decryption plus set operations, optional Secrets Manager, AppConfig, and DynamoDB providers, force-fetch support, JSON transforms, base64 binary transforms, suffix-based auto transforms, and initial docs/snippet | Broader AWS provider examples |
| Parser | `EventParser`, `ParsedEvent`, `ParseError`, serde JSON string/slice/value parsing, Transfer Family authorizer event/response models, AppSync Events model and publish payload envelope, Bedrock Agent OpenAPI event model and input text envelope, Bedrock Agent function-details model and input text envelope, DynamoDB stream on-failure destination model, S3 EventBridge notification model, IoT Core registry event models, Cognito migrate-user and custom sender event models, optional `aws_lambda_events` API Gateway REST/HTTP/WebSocket API body, AppSync direct resolver arguments/source, Bedrock Agent OpenAPI input text, ALB target group body, Lambda Function URL body, VPC Lattice v1/v2 body, EventBridge detail, CloudFormation custom resource properties, Cognito User Pool user attributes, SQS body, SNS message, SNS-over-SQS message, S3 record, S3-over-SQS record, S3 Object Lambda configuration payload, S3 Batch job task, SES record, CloudWatch Logs message, Kinesis record data, Kinesis-delivered DynamoDB stream image, Firehose record data, Firehose-delivered SQS body, DynamoDB stream image, Kafka record value envelopes, initial docs/snippet, and API Gateway/EventBridge/SQS/ALB/Lambda Function URL/SNS/S3/S3 Object Lambda/S3 Batch/S3-over-SQS/SNS-over-SQS/SES/CloudFormation/stream JSON fixtures | Broader `aws_lambda_events` envelopes, Powertools adapters, shared event fixtures, schema-aware parsing |
| Batch | `BatchRecord`, `BatchProcessor`, `BatchProcessingReport`, `BatchRecordResult`, `BatchItemFailure`, `BatchResponse`, sequential and concurrent generic processing, stream checkpoint helpers, optional `aws_lambda_events` SQS, Kinesis, and DynamoDB stream adapters, SQS FIFO early-stop behavior, optional parser-integrated SQS message body, Kinesis record data, and DynamoDB stream image processing, and initial docs/snippet | Larger examples |
| Validation | `Validator`, `Validate`, `ValidationError`, required text, length, range, custom predicate helpers, inbound/outbound validation wrappers, optional local JSON Schema backend, compiled schema cache, event-handler validation hooks, and initial docs/snippet | Richer handler examples |
| Idempotency | `IdempotencyConfig`, `IdempotencyKey`, `Idempotency`, `AsyncIdempotency`, `IdempotencyOutcome`, typed workflow errors, SHA-256 JSON payload hashing, JSON Pointer key extraction, sync and async handler wrappers, payload hash validation, Lambda remaining-time in-progress expiry, result replay, sync and async store traits/errors/results, in-memory store, local cache wrapper, optional DynamoDB store, and initial docs/snippet | Broader Lambda and DynamoDB examples |
| Feature flags | `FeatureFlagConfig`, `FeatureFlag`, `FeatureRule`, `FeatureCondition`, `RuleAction`, `FeatureFlagCachePolicy`, `FeatureFlags`, `AsyncFeatureFlags`, sync/async store traits, `InMemoryFeatureFlagStore`, optional `AppConfigFeatureFlagStore`, boolean and JSON-valued evaluation, enabled-feature listing, configuration cache policies, common context comparators, modulo range matching, time-window rules, and initial docs/snippet | Richer examples |
| Event handler | `Method`, method parsing/matching, `Request`, `Response`, `HttpError`, `PathParams`, `Route`, `AsyncRoute`, `Router`, `AsyncRouter`, static/dynamic path precedence, `ANY` and multi-method route registration, 404 dispatch, unsupported-method `405` adapter responses, custom not-found handlers, fallible sync/async route handlers with router-level and typed error handlers, router composition with path prefixes, router-level and route-specific request/response middleware, request-scoped and shared typed extensions, `CorsConfig`, preflight responses, routed/404/error CORS headers, optional router-level and route-specific validation hooks, optional gzip/deflate compression middleware, optional sync/async AppSync direct and batch resolver routing and composition, optional AppSync Events routing and composition, optional Bedrock Agent OpenAPI adapter, optional sync/async Bedrock Agent function-details resolver, optional ALB, Lambda Function URL, and VPC Lattice adapters, optional API Gateway REST API v1 / HTTP API v2 / WebSocket API adapters, and initial docs/snippet | Additional resolver families |
| Testing | `LambdaContextStub`, parameter provider stub re-export, text/bytes fixture readers, and JSON fixture decoder | Fake AWS providers, handler harnesses |

## Next Durable Work

The next durable work should turn the landed primitives into Lambda-facing utilities:

1. Add production OpenTelemetry exporter examples that wire tracer export helpers into OTLP or vendor providers.
2. Expand parameter provider docs and examples. Keep AWS SDK dependencies aligned with the documented MSRV.
3. Expand parser envelopes and fixtures using `aws_lambda_events` as the default event model source.
4. Expand idempotency examples where AWS retry semantics overlap.
5. Add event-handler adapters for additional resolver families and document the current HTTP, WebSocket, ALB, Lambda
   Function URL, VPC Lattice, AppSync, AppSync Events, Bedrock Agent OpenAPI, and Bedrock Agent function-details
   surfaces.

## Crate Strategy

| Crate | Current role | Notes |
| --- | --- | --- |
| `aws-lambda-powertools` | Primary user-facing crate | Depends on support crates through optional dependencies and re-exports enabled utilities |
| `aws-lambda-powertools-core` | Shared foundations | Keep small: config, env, cold start, metadata, and other genuine cross-crate foundations |
| `aws-lambda-powertools-logger` | Structured logs | JSON renderer, sampling, correlation IDs, Lambda context fields, key redaction, custom formatter/redaction hooks, optional `tracing` subscriber layer, and initial docs/snippet exist; next work is broader handler examples |
| `aws-lambda-powertools-metrics` | CloudWatch EMF metrics | Renderer, flush API, high-resolution metrics, default dimensions, explicit timestamps, overflow flush helpers, async capture helpers, and initial docs/snippet exist; next work is broader handler examples |
| `aws-lambda-powertools-tracer` | Tracing facade | Segment records, X-Ray header propagation helpers, optional X-Ray-compatible subsegment document rendering, optional X-Ray daemon UDP transport, optional `tracing` span conversion, optional OpenTelemetry span builder/attribute export, and a buildable OpenTelemetry SDK/stdout exporter snippet exist; next work is production OTLP/OpenTelemetry deployment examples |
| `aws-lambda-powertools-parameters` | Parameter retrieval | Sync and async traits, cache facades, async provider/retrieval errors, in-memory provider, optional SSM single-parameter, by-name, and path providers plus set operations, optional Secrets Manager, AppConfig, and DynamoDB providers, force-fetch support, JSON/base64/auto transforms, and initial docs/snippet exist; broader AWS provider examples are next |
| `aws-lambda-powertools-parser` | Event parsing | serde JSON facade plus Transfer Family authorizer event/response, AppSync Events, Bedrock Agent OpenAPI, Bedrock Agent function-details, DynamoDB stream on-failure destination, S3 EventBridge notification, IoT Core registry, Cognito migrate-user, and Cognito custom sender event models, API Gateway REST API, HTTP API, and WebSocket API, AppSync direct resolver arguments/source, AppSync Events publish payload, Bedrock Agent OpenAPI input text, ALB, Lambda Function URL, VPC Lattice, SQS, SNS, SNS-over-SQS, S3, S3-over-SQS, S3 Object Lambda, S3 Batch, EventBridge, CloudFormation custom resource properties, Cognito User Pool trigger user attributes, SES, CloudWatch Logs, Kinesis, Kinesis-delivered DynamoDB stream image, Firehose, Firehose-delivered SQS, DynamoDB stream image, Kafka `aws_lambda_events` envelopes, initial docs/snippet, and API Gateway/EventBridge/SQS/ALB/Lambda Function URL/SNS/S3/S3 Object Lambda/S3 Batch/S3-over-SQS/SNS-over-SQS/SES/CloudFormation/stream JSON fixtures exist; broader envelope coverage and fixtures are next |
| `aws-lambda-powertools-batch` | Partial batch responses | Generic sequential/concurrent processing, stream checkpoint helpers, SQS, Kinesis, and DynamoDB stream adapters, parser-integrated SQS message body, Kinesis record data, and DynamoDB stream image processing, and initial docs/snippet exist; larger examples are next |
| `aws-lambda-powertools-idempotency` | Deduplication | JSON payload hashing, key extraction, sync and async handler workflows, Lambda remaining-time in-progress expiry, replay, records, in-memory store, local cache wrapper, optional DynamoDB persistence, and initial docs/snippet exist; broader Lambda and DynamoDB examples are next |
| `aws-lambda-powertools-validation` | Payload validation | Basic validators, inbound/outbound wrappers, optional JSON Schema validation, schema caching, event-handler validation hooks, and initial docs/snippet exist; next work is richer handler examples |
| `aws-lambda-powertools-feature-flags` | Feature flag evaluation | Typed configuration, sync/async rule evaluation, in-memory and optional AppConfig stores, boolean/JSON-valued flags, enabled-feature listing, cache policies, common comparators, modulo matching, time-window rule actions, and initial docs/snippet exist; next work is richer examples |
| `aws-lambda-powertools-event-handler` | Routing | Dependency-free sync/async routing, multi-method route registration, built-in HTTP errors, unsupported-method `405` adapter responses, custom not-found handlers, fallible sync/async route error handling with typed handlers, router composition with path prefixes, router-level and route-specific middleware, request-scoped and shared typed extensions, CORS, optional router-level and route-specific validation hooks, optional compression middleware, optional sync/async AppSync direct and batch resolver routing and composition, optional AppSync Events routing and composition, optional Bedrock Agent OpenAPI adapter, optional sync/async Bedrock Agent function-details resolver, optional ALB, Lambda Function URL, VPC Lattice, API Gateway REST API, HTTP API, and WebSocket API adapters, and initial docs/snippet exist; next work is additional event adapters |
| `aws-lambda-powertools-testing` | Test helpers | Context stubs, parameter provider stubs, and fixture loaders exist; expand fake providers and handler harnesses only as real utilities need them |

Provider features should live on the owning utility crate first and be re-exposed by the umbrella crate only when that is
ergonomic for users.

## Feature Flags

Implemented umbrella features:

- `logger`
- `logger-tracing`
- `metrics`
- `tracer`
- `tracer-opentelemetry`
- `tracer-tracing`
- `tracer-xray`
- `tracer-xray-daemon`
- `parameters`
- `parameters-appconfig`
- `parameters-dynamodb`
- `parameters-secrets`
- `parameters-ssm`
- `parser`
- `parser-aws-lambda-events`
- `batch`
- `batch-aws-lambda-events`
- `batch-parser`
- `idempotency`
- `idempotency-dynamodb`
- `feature-flags`
- `feature-flags-appconfig`
- `validation`
- `validation-jsonschema`
- `event-handler`
- `event-handler-appsync-events`
- `event-handler-bedrock-agent-functions`
- `event-handler-compression`
- `event-handler-validation`
- `event-handler-aws-lambda-events`
- `all`

Likely future provider and integration features:

- `idempotency-redis`
- `parser-serde`
- `parser-schemars`
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
- [x] Implement `tracing` span integration.
- [x] Add X-Ray trace header rendering helpers.
- [x] Add optional X-Ray-compatible tracer subsegment document rendering.
- [x] Add optional X-Ray daemon transport feature.
- [x] Add optional OpenTelemetry span builder and attribute export helpers.
- [x] Add buildable OpenTelemetry SDK/stdout exporter tracer snippet.
- [x] Add parameter force-fetch and local/auto value transforms.
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
- [x] Add Bedrock Agent OpenAPI parser model and input text envelope.
- [x] Add Bedrock Agent function-details parser model and input text envelope.
- [x] Add ALB target group parser body envelope based on `aws_lambda_events`.
- [x] Add Lambda Function URL parser body envelope based on `aws_lambda_events`.
- [x] Add VPC Lattice v1/v2 parser body envelopes based on `aws_lambda_events`.
- [x] Add Kinesis and Firehose parser record-data envelopes based on `aws_lambda_events`.
- [x] Add DynamoDB stream parser image envelopes based on `aws_lambda_events`.
- [x] Add CloudWatch Logs parser message envelope based on `aws_lambda_events`.
- [x] Add Kafka parser record-value envelopes based on `aws_lambda_events`.
- [x] Add S3, S3-over-SQS, and SNS-over-SQS parser envelopes based on `aws_lambda_events`.
- [x] Add CloudFormation custom resource property parser envelopes based on `aws_lambda_events`.
- [x] Add SES parser record envelopes based on `aws_lambda_events`.
- [x] Add S3 Object Lambda and S3 Batch parser envelopes based on `aws_lambda_events`.
- [x] Add Transfer Family authorizer event and response models.
- [x] Add Cognito User Pool trigger user-attribute parser envelopes based on `aws_lambda_events`.
- [x] Add Firehose-delivered SQS parser envelopes based on `aws_lambda_events`.
- [x] Add Kinesis-delivered DynamoDB stream parser envelopes based on `aws_lambda_events`.
- [x] Add DynamoDB stream Lambda on-failure destination model.
- [x] Add IoT Core registry event models.
- [x] Add Cognito custom email and SMS sender event models.
- [x] Add Cognito migrate-user event model with SMS MFA response support.
- [x] Add AppSync Events model and publish payload parser envelope.
- [x] Add S3 EventBridge notification event model.
- [x] Add parser feature doc and buildable snippet.
- [x] Add initial parser event fixtures for API Gateway v2, EventBridge, and SQS.
- [x] Add parser stream event fixtures for Kinesis, Firehose, CloudWatch Logs, and DynamoDB.
- [x] Add parser HTTP/message event fixtures for ALB, Lambda Function URL, and SNS.
- [x] Add parser storage and custom-resource event fixtures for S3, SES, and CloudFormation.
- [x] Add parser nested notification fixtures for S3-over-SQS and SNS-over-SQS.
- [x] Add parser S3 Object Lambda and S3 Batch fixtures.
- [ ] Expand parser envelopes and fixtures based on `aws_lambda_events`.
- [x] Add SQS source-specific batch processing and FIFO retry semantics.
- [x] Add Kinesis and DynamoDB stream batch processors and retry semantics.
- [x] Add generic concurrent batch processing.
- [x] Add stream checkpoint helpers for Kinesis and DynamoDB retry semantics.
- [x] Add batch feature doc and buildable snippet.
- [x] Add parser-integrated SQS batch processing.
- [x] Add parser-integrated Kinesis batch processing.
- [x] Add parser-integrated DynamoDB stream image batch processing.
- [x] Add JSON Schema validation behind an optional feature.
- [x] Add validation schema cache and inbound/outbound wrappers.
- [x] Add event-handler validation hooks.
- [x] Add event-handler route-specific validation hooks.
- [x] Add idempotency handler workflow, key hashing, payload validation, and replay behavior.
- [x] Add async idempotency store and handler workflow.
- [x] Add DynamoDB idempotency persistence and provider-level concurrency semantics.
- [x] Add Lambda remaining-time handling for in-progress idempotency expiry.
- [x] Add local cache wrapper for idempotency stores.
- [x] Add first-pass feature flag schema parsing and rule evaluation.
- [x] Add AppConfig-backed feature flag store.
- [x] Add feature flag cache policy support.
- [x] Add feature flag time-window rule actions.
- [x] Add API Gateway REST API and HTTP API adapters for event-handler routing.
- [x] Add event-handler CORS configuration and preflight handling.
- [x] Add event-handler custom not-found handlers.
- [x] Add event-handler fallible route handlers and router-level error handlers.
- [x] Add event-handler typed fallible route error handlers.
- [x] Add built-in HTTP errors for fallible event-handler routes.
- [x] Add unsupported-method `405` responses for event-handler HTTP adapters.
- [x] Add event-handler multi-method route registration helpers.
- [x] Add event-handler router composition with path prefixes.
- [x] Add event-handler route-specific request/response middleware.
- [x] Add event-handler request-scoped and shared typed extensions.
- [x] Add AppSync direct resolver composition.
- [x] Add AppSync Events resolver composition.
- [x] Add event-handler request/response middleware.
- [x] Add event-handler async handlers and related HTTP routing integrations.
- [x] Add event-handler gzip/deflate response compression middleware.
- [x] Add event-handler sync and async AppSync direct and batch resolver routing.
- [x] Add event-handler AppSync Events publish/subscribe routing.
- [x] Add event-handler Bedrock Agent adapter.
- [x] Add event-handler sync and async Bedrock Agent function-details resolver.
- [x] Add event-handler ALB adapter.
- [x] Add event-handler Lambda Function URL adapter.
- [x] Add event-handler VPC Lattice v1/v2 adapters.
- [x] Add event-handler API Gateway WebSocket adapter.
- [x] Add event-handler feature doc and buildable snippet.
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
- Tracing: build on Rust `tracing` spans first, then add exporter-neutral OpenTelemetry conversion helpers without
  selecting a global SDK or exporter by default.
- Contributor commands: keep plain Cargo commands as the canonical workflow. Add `just` or `make` only later as optional
  convenience wrappers.
- Lockfile: keep `Cargo.lock` committed for reproducible workspace and example validation.
