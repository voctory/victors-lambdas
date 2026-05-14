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
| `POWERTOOLS_TRACE_DISABLED` | Disables trace data collection when set to a truthy value. |
| `POWERTOOLS_TRACER_CAPTURE_RESPONSE` | Enables captured handler responses. |
| `POWERTOOLS_TRACER_CAPTURE_ERROR` | Enables captured handler errors. |
| `AWS_XRAY_DAEMON_ADDRESS` | Sets the UDP daemon address used by `tracer-xray-daemon`. |

## Supported Behavior

- X-Ray trace header parsing and rendering.
- Segment records with annotations and metadata.
- Optional response and error capture.
- JSON-compatible trace values with deterministic field ordering.
- Optional `tracing` span creation through `tracer-tracing`.
- Optional OpenTelemetry span builder and attribute export through `tracer-opentelemetry`.
- Optional X-Ray-compatible subsegment document rendering through `tracer-xray`.
- Optional X-Ray daemon UDP transport through `tracer-xray-daemon`.

## OpenTelemetry Export Helpers

Enable `tracer-opentelemetry` to convert a segment into OpenTelemetry types:

```toml
aws-lambda-powertools = { version = "0.1", features = ["tracer-opentelemetry"] }
```

`TraceSegment::to_otel_span_builder` creates an OpenTelemetry `SpanBuilder` with the segment name and attributes.
`TraceSegment::to_otel_attributes` returns the same attributes as `KeyValue` pairs, and
`TraceSegment::record_otel_attributes` records them onto an existing OpenTelemetry span. The crate does not install a
global provider or choose an OTLP exporter; wire the returned values into the OpenTelemetry SDK/exporter that fits your
Lambda runtime setup.

The buildable tracer snippet shows this boundary with `opentelemetry_sdk` and the stdout span exporter. Use it as a
shape for custom exporters by replacing the exporter passed to `SdkTracerProvider::builder`.

The snippet also includes an OTLP/HTTP protobuf provider. Set `OTEL_EXPORTER_OTLP_TRACES_ENDPOINT` when you want to send
the example span to a collector or collector Lambda extension:

```sh
OTEL_EXPORTER_OTLP_TRACES_ENDPOINT=http://localhost:4318 cargo run -p tracer-snippet
```

The endpoint block is skipped when the environment variable is not set, so the snippet remains runnable without a local
collector.

## X-Ray Documents

Enable `tracer-xray` to render a segment as an X-Ray-compatible subsegment document:

```toml
aws-lambda-powertools = { version = "0.1", features = ["tracer-xray"] }
```

The renderer requires a trace id and parent id from the active X-Ray header. The caller supplies the subsegment id and
epoch-second start/end timestamps so the crate does not introduce hidden global state, a random ID generator, or a
process-wide clock.

## X-Ray Daemon Transport

Enable `tracer-xray-daemon` to send rendered X-Ray documents to the local daemon over UDP:

```toml
aws-lambda-powertools = { version = "0.1", features = ["tracer-xray-daemon"] }
```

`XrayDaemonClient::from_env` reads `AWS_XRAY_DAEMON_ADDRESS` and falls back to `127.0.0.1:2000`. Addresses in X-Ray SDK
format, such as `tcp:127.0.0.1:2000 udp:127.0.0.1:2000`, use the UDP endpoint. Use `send_document` for an already
rendered document, or `send_subsegment` to render a `TraceSegment` and send it in one call.

## Snippet

The buildable snippet in [examples/snippets/tracer/src/main.rs](../../examples/snippets/tracer/src/main.rs) parses an
X-Ray header, records annotations and metadata, captures a response, renders a subsegment document, converts the segment
into OpenTelemetry SDK spans with stdout and optional OTLP exporters, and configures the daemon client.

Run it locally with:

```sh
cargo run -p tracer-snippet
```

Use the `tracer-tracing` feature when you want to create `tracing::Span` values from the same tracer configuration.
