# Validation

The validation utility provides local validation helpers for decoded payloads and optional JSON Schema validation. It is
exposed through the `validation` Cargo feature on the umbrella crate:

```toml
aws-lambda-powertools = { version = "0.1", features = ["validation"] }
```

## Supported Behavior

- `Validate` trait integration for request and response types.
- Inbound and outbound validation wrappers that return the original value when it passes.
- Required text, text length, numeric range, and custom predicate helpers.
- Structured validation errors with kind, field, and message accessors.
- Optional JSON Schema validation and compiled schema caching through `validation-jsonschema`.
- Event-handler validation hooks when combined with the `event-handler-validation` feature.

## JSON Schema

Enable `validation-jsonschema` to validate `serde_json::Value` payloads against in-memory JSON Schema documents:

```toml
aws-lambda-powertools = { version = "0.1", features = ["validation-jsonschema"] }
```

`JsonSchemaCache` lets Lambda handlers compile schemas once during initialization or first use and reuse them across warm
invocations. Remote reference resolution is intentionally not enabled by default.

## Snippet

The buildable snippet in [examples/snippets/validation/src/main.rs](../../examples/snippets/validation/src/main.rs)
validates a Rust request type with the `Validate` trait and then validates the same payload against a cached JSON Schema.

Run it locally with:

```sh
cargo run -p validation-snippet
```

Use `Validator::validate_inbound` before business logic and `Validator::validate_outbound` before serializing a
response when a handler pipeline needs explicit request or response guarantees.
