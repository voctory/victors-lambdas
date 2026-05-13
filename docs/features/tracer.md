# Tracer

The tracer utility records trace context, annotations, metadata, captured responses, and captured errors in a Rust-owned
segment value. It is exposed through the `tracer` Cargo feature on the umbrella crate:

```toml
aws-lambda-powertools = { version = "0.1", features = ["tracer"] }
```

## Configuration

Use `TracerConfig::new` for explicit service configuration or `TracerConfig::from_env` to read Powertools-compatible
environment variables.

| Environment variable | Effect |
| --- | --- |
| `POWERTOOLS_SERVICE_NAME` | Sets the service namespace used by tracer-created segments. |
| `POWERTOOLS_TRACE_ENABLED` | Enables or disables trace data collection. |
| `POWERTOOLS_TRACER_CAPTURE_RESPONSE` | Enables captured handler responses. |
| `POWERTOOLS_TRACER_CAPTURE_ERROR` | Enables captured handler errors. |

## Supported Behavior

- X-Ray trace header parsing and rendering.
- Segment records with annotations and metadata.
- Optional response and error capture.
- JSON-compatible trace values with deterministic field ordering.
- Optional `tracing` span creation through `tracer-tracing`.
- Optional X-Ray-compatible subsegment document rendering through `tracer-xray`.

## X-Ray Documents

Enable `tracer-xray` to render a segment as an X-Ray-compatible subsegment document:

```toml
aws-lambda-powertools = { version = "0.1", features = ["tracer-xray"] }
```

The renderer requires a trace id and parent id from the active X-Ray header. The caller supplies the subsegment id and
epoch-second start/end timestamps so the crate does not introduce hidden global state, a random ID generator, or a
process-wide clock.

## Snippet

The buildable snippet in [examples/snippets/tracer/src/main.rs](../../examples/snippets/tracer/src/main.rs) parses an
X-Ray header, records annotations and metadata, captures a response, and renders a subsegment document.

Run it locally with:

```sh
cargo run -p tracer-snippet
```

Use the `tracer-tracing` feature when you want to create `tracing::Span` values from the same tracer configuration.
