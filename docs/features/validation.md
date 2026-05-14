# Validation

The validation utility provides local validation helpers for decoded payloads and optional JSON Schema validation. It is
exposed through the `validation` Cargo feature on the umbrella crate:

```toml
victors-lambdas = { version = "0.1", features = ["validation"] }
```

## Supported Behavior

- `Validate` trait integration for request and response types.
- Inbound and outbound validation wrappers that return the original value when it passes.
- Required text, text length, numeric range, and custom predicate helpers.
- Structured validation errors with kind, field, and message accessors.
- Optional JSON Schema validation and compiled schema caching through `validation-jsonschema`.
- Optional JMESPath envelope extraction before JSON Schema validation through `validation-jmespath`.
- Event-handler validation hooks when combined with the `event-handler-validation` feature.

## JSON Schema

Enable `validation-jsonschema` to validate `serde_json::Value` payloads against in-memory JSON Schema documents:

```toml
victors-lambdas = { version = "0.1", features = ["validation-jsonschema"] }
```

`JsonSchemaCache` lets Lambda handlers compile schemas once during initialization or first use and reuse them across warm
invocations. Remote reference resolution is intentionally not enabled by default.

Enable `validation-jmespath` to select the JSON value to validate from a Lambda event envelope. This also enables JSON
Schema validation and the Powertools JMESPath helper functions:

```toml
victors-lambdas = { version = "0.1", features = ["validation-jmespath"] }
```

```rust
# use victors_lambdas::prelude::Validator;
# let schema = serde_json::json!({"type": "object"});
# let event = serde_json::json!({"body": "{}"});
Validator::new().json_schema_envelope(&schema, &event, "powertools_json(body)")?;
# Ok::<(), victors_lambdas::prelude::ValidationError>(())
```

## Snippet

The buildable snippet in [examples/snippets/validation/src/main.rs](../../examples/snippets/validation/src/main.rs)
validates a Rust request type with the `Validate` trait and then validates an API Gateway-style body envelope against a
cached JSON Schema.

Run it locally with:

```sh
cargo run -p validation-snippet
```

Use `Validator::validate_inbound` before business logic and `Validator::validate_outbound` before serializing a
response when a handler pipeline needs explicit request or response guarantees.
