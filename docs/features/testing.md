# Testing

The testing crate provides small test doubles and fixture helpers for Lambda-oriented unit tests.

```toml
victors-lambdas-testing = { version = "0.1" }
```

## Supported Behavior

- `LambdaContextStub` for stable request ID and function name assertions.
- `HandlerHarness` for invoking sync or async handler-shaped functions with a reusable context.
- `ParameterProviderStub`, re-exported from the parameters crate for in-memory parameter tests.
- Optional `FeatureFlagStoreStub` for in-memory feature flag tests.
- Optional `IdempotencyStoreStub` for in-memory idempotency tests.
- Optional `S3ObjectClientStub` for testing streaming code against in-memory S3 objects.
- Text, bytes, and JSON fixture loaders.

Enable the feature-specific stubs you need:

```toml
victors-lambdas-testing = { version = "0.1", features = ["feature-flags", "idempotency", "streaming"] }
```

## Handler Harness

Use `HandlerHarness` when the handler is a plain Rust function that accepts a typed event and context reference:

```rust
use victors_lambdas_testing::HandlerHarness;

#[derive(serde::Deserialize)]
struct OrderEvent {
    order_id: String,
}

let harness = HandlerHarness::default();
let output = harness.invoke(OrderEvent { order_id: "order-1".into() }, |event, context| {
    format!("{}:{}", context.request_id(), event.order_id)
});

assert_eq!(output, "test-request-id:order-1");
```

For fixture-driven tests, let the harness decode JSON before invoking the handler:

```rust,no_run
use std::path::Path;

use victors_lambdas_testing::HandlerHarness;

#[derive(serde::Deserialize)]
struct OrderEvent {
    order_id: String,
}

let harness = HandlerHarness::default();
let output = harness.invoke_json(Path::new("tests/events/order.json"), |event: OrderEvent, _| {
    event.order_id
})?;

# Ok::<(), victors_lambdas_testing::FixtureError>(())
```

## Store Stubs

`FeatureFlagStoreStub` and `IdempotencyStoreStub` re-export the corresponding utility crates' in-memory stores under
testing-oriented names:

```rust
use victors_lambdas_feature_flags::{FeatureFlag, FeatureFlagConfig, FeatureFlagContext, FeatureFlags};
use victors_lambdas_testing::FeatureFlagStoreStub;

let store = FeatureFlagStoreStub::from_config(
    FeatureFlagConfig::new().with_feature("beta", FeatureFlag::boolean(true)),
);
let flags = FeatureFlags::new(store);

assert!(flags.evaluate_bool("beta", &FeatureFlagContext::new(), false)?);

# Ok::<(), victors_lambdas_feature_flags::FeatureFlagError>(())
```

## S3 Stub

`S3ObjectClientStub` implements the streaming crate's `S3ObjectClient` trait and records range requests:

```rust,no_run
use std::io::{Read as _, Seek as _, SeekFrom};

use victors_lambdas_streaming::S3Object;
use victors_lambdas_testing::S3ObjectClientStub;

let client = S3ObjectClientStub::new().with_object("orders", "order.json", b"{\"id\":1}");
let mut object = S3Object::for_bucket_key("orders", "order.json", client);

object.seek(SeekFrom::Start(2))?;

let mut body = String::new();
object.read_to_string(&mut body)?;

assert_eq!(
    object.source_ref().client().range_requests()[0].range_header(),
    "bytes=2-"
);

# Ok::<(), std::io::Error>(())
```
