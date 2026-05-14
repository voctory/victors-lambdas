# Idempotency

The idempotency utility coordinates retries so repeated Lambda invocations with the same logical request can replay a
stored response instead of running the handler again. It is exposed through the `idempotency` Cargo feature on the
umbrella crate:

```toml
aws-lambda-powertools = { version = "0.1", features = ["idempotency"] }
```

Enable `idempotency-jmespath` when keys should come from a JMESPath expression, including Powertools envelope helpers:

```toml
aws-lambda-powertools = { version = "0.1", features = ["idempotency-jmespath"] }
```

## Configuration

Use `IdempotencyConfig::from_env` to honor Powertools-compatible environment variables or `IdempotencyConfig::new` for
explicit configuration.

| Environment variable | Effect |
| --- | --- |
| `POWERTOOLS_IDEMPOTENCY_DISABLED` | Disables idempotency when truthy. |

Configuration also controls completed-record TTL, in-progress TTL, an optional key prefix, and the current Lambda
deadline. Register the invocation deadline or remaining time before each invocation when reusing a workflow across warm
Lambda invocations.

## Stores

`InMemoryIdempotencyStore` is useful for tests, examples, and local workflows. Enable `idempotency-dynamodb` on the
umbrella crate to use `DynamoDbIdempotencyStore` with an AWS SDK `DynamoDB` client:

```toml
aws-lambda-powertools = { version = "0.1", features = ["idempotency-dynamodb"] }
```

The DynamoDB store defaults to a string partition key named `id`; composite-key tables can set a sort key attribute and
static partition key value.

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
- Payload hash validation for replay safety.
- Completed-response replay.
- In-progress record rejection.
- Handler failure cleanup so later retries can proceed.
- Lambda remaining-time based in-progress expiry.
- In-memory and optional DynamoDB stores.
- Optional local cache wrapper for sync and async stores.

## Snippet

The buildable snippet in [examples/snippets/idempotency/src/main.rs](../../examples/snippets/idempotency/src/main.rs)
uses `powertools_json(body).request_id` to derive the idempotency key from an API Gateway-style envelope, executes the
first request, stores the record through a local cache wrapper, and replays the stored response for the second request.

Run it locally with:

```sh
cargo run -p idempotency-snippet
```

Use `execute_json_with_key` when a stable field such as a request ID should identify the operation while the full
payload hash is still validated. Use `key_from_jmespath` for envelope-aware key selection, `key_from_json_pointer` for
direct JSON Pointer selection, and `execute_json` when hashing the whole payload is the intended key strategy.

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
