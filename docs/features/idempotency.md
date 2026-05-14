# Idempotency

The idempotency utility coordinates retries so repeated Lambda invocations with the same logical request can replay a
stored response instead of running the handler again. It is exposed through the `idempotency` Cargo feature on the
umbrella crate:

```toml
aws-lambda-powertools = { version = "0.1", features = ["idempotency"] }
```

Enable `idempotency-jmespath` when keys or payload validation hashes should come from a JMESPath expression, including
Powertools envelope helpers:

```toml
aws-lambda-powertools = { version = "0.1", features = ["idempotency-jmespath"] }
```

## Configuration

Use `IdempotencyConfig::from_env` to honor Powertools-compatible environment variables or `IdempotencyConfig::new` for
explicit configuration.

| Environment variable | Effect |
| --- | --- |
| `POWERTOOLS_IDEMPOTENCY_DISABLED` | Disables idempotency when truthy. |

Configuration also controls completed-record TTL, in-progress TTL, an optional key prefix, the payload validation
strategy, and the current Lambda deadline. Register the invocation deadline or remaining time before each invocation
when reusing a workflow across warm Lambda invocations.

Full-payload validation is enabled by default. Use `without_payload_validation` only when replay safety is handled
outside the idempotency record. With `idempotency-jmespath`, use `with_payload_validation_jmespath` to hash a stable
business payload while ignoring retry-varying envelope fields:

```rust
# use aws_lambda_powertools::prelude::IdempotencyConfig;
let config = IdempotencyConfig::from_env()
    .with_key_prefix("checkout")
    .with_payload_validation_jmespath("powertools_json(body)");
```

## Stores

`InMemoryIdempotencyStore` is useful for tests, examples, and local workflows. Enable `idempotency-dynamodb` on the
umbrella crate to use `DynamoDbIdempotencyStore` with an AWS SDK `DynamoDB` client:

```toml
aws-lambda-powertools = { version = "0.1", features = ["idempotency-dynamodb"] }
```

The DynamoDB store defaults to a string partition key named `id`; composite-key tables can set a sort key attribute and
static partition key value.

Use `CacheIdempotencyStore` with an `AsyncIdempotencyCacheClient` implementation for Redis, Valkey, or another external
TTL cache. The adapter stores opaque idempotency record bytes and lets the cache service expire entries.

Wrap a durable store in `CachedIdempotencyStore` to keep recently read and written records in the current Lambda
execution environment:

```rust
# use aws_lambda_powertools::prelude::{CachedIdempotencyStore, InMemoryIdempotencyStore};
let store = CachedIdempotencyStore::new(InMemoryIdempotencyStore::new());
```

The local cache is an optimization only. Use a persistent backing store for correctness across concurrent invocations,
cold starts, and different execution environments.

## Supported Behavior

- Sync and async idempotency workflows.
- Payload-derived keys, JSON Pointer key extraction, and optional JMESPath key extraction.
- Full-payload validation, optional JMESPath payload validation selection, and opt-out validation for explicit cases.
- Completed-response replay.
- In-progress record rejection.
- Handler failure cleanup so later retries can proceed.
- Lambda remaining-time based in-progress expiry.
- In-memory, generic external TTL cache, and optional DynamoDB stores.
- Optional local cache wrapper for sync and async stores.

## Snippet

The buildable snippet in [examples/snippets/idempotency/src/main.rs](../../examples/snippets/idempotency/src/main.rs)
uses `powertools_json(body).request_id` to derive the idempotency key from an API Gateway-style envelope, validates the
stable `body` payload with JMESPath while ignoring retry-varying envelope metadata, stores the record through a local
cache wrapper, and replays the stored response for the second request.

Run it locally with:

```sh
cargo run -p idempotency-snippet
```

Use `execute_json_with_key` when a stable field such as a request ID should identify the operation. Use
`with_payload_validation_jmespath` when only a stable payload subset should be validated, `key_from_jmespath` for
envelope-aware key selection, `key_from_json_pointer` for direct JSON Pointer selection, and `execute_json` when hashing
the whole payload is the intended key strategy.

## AWS Store Snippet

The buildable AWS-backed snippet in
[examples/snippets/idempotency-aws/src/main.rs](../../examples/snippets/idempotency-aws/src/main.rs) shows an async
workflow backed by `DynamoDbIdempotencyStore`, wrapped in `CachedIdempotencyStore`, with a per-invocation Lambda
remaining-time setting.

The snippet is guarded so local validation does not call AWS by default:

```sh
cargo run -p idempotency-aws-snippet
RUN_AWS_IDEMPOTENCY_SNIPPET=1 IDEMPOTENCY_TABLE=powertools-idempotency cargo run -p idempotency-aws-snippet
```

Set `RUN_AWS_IDEMPOTENCY_SNIPPET=1` only in an environment with AWS credentials, region configuration, and a DynamoDB
table that matches the configured idempotency attributes.
