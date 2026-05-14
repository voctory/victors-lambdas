# Testing

The testing crate provides small test doubles and fixture helpers for Lambda-oriented unit tests.

```toml
aws-lambda-powertools-testing = { version = "0.1" }
```

## Supported Behavior

- `LambdaContextStub` for stable request ID and function name assertions.
- `HandlerHarness` for invoking sync or async handler-shaped functions with a reusable context.
- `ParameterProviderStub`, re-exported from the parameters crate for in-memory parameter tests.
- Optional `S3ObjectClientStub` for testing streaming code against in-memory S3 objects.
- Text, bytes, and JSON fixture loaders.

Enable `streaming` to use the S3 object client stub:

```toml
aws-lambda-powertools-testing = { version = "0.1", features = ["streaming"] }
```

## Handler Harness

Use `HandlerHarness` when the handler is a plain Rust function that accepts a typed event and context reference:

```rust
use aws_lambda_powertools_testing::HandlerHarness;

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

use aws_lambda_powertools_testing::HandlerHarness;

#[derive(serde::Deserialize)]
struct OrderEvent {
    order_id: String,
}

let harness = HandlerHarness::default();
let output = harness.invoke_json(Path::new("tests/events/order.json"), |event: OrderEvent, _| {
    event.order_id
})?;

# Ok::<(), aws_lambda_powertools_testing::FixtureError>(())
```

## S3 Stub

`S3ObjectClientStub` implements the streaming crate's `S3ObjectClient` trait and records range requests:

```rust,no_run
use std::io::{Read as _, Seek as _, SeekFrom};

use aws_lambda_powertools_streaming::S3Object;
use aws_lambda_powertools_testing::S3ObjectClientStub;

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
