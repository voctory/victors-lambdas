# Metrics

The metrics utility renders CloudWatch Embedded Metric Format JSON and can write one EMF line to stdout. It is exposed
through the `metrics` Cargo feature on the umbrella crate:

```toml
victors-lambdas = { version = "0.1", features = ["metrics"] }
```

## Configuration

Use `MetricsConfig::new` for explicit service and namespace configuration or `MetricsConfig::from_env` to read
Powertools-compatible environment variables.

| Environment variable | Effect |
| --- | --- |
| `POWERTOOLS_SERVICE_NAME` | Sets the default `service` dimension. |
| `POWERTOOLS_METRICS_NAMESPACE` | Sets the CloudWatch namespace. |
| `POWERTOOLS_METRICS_DISABLED` | Disables EMF output when truthy. |
| `POWERTOOLS_DEV` | Disables EMF output when `POWERTOOLS_METRICS_DISABLED` is not set. |

## Supported Behavior

- Standard and high-resolution metric data points.
- Request-scoped dimensions and persistent default dimensions.
- Top-level EMF metadata.
- CloudWatch EMF name, dimension, metadata, and metric count validation.
- Cold-start metric support.
- Explicit timestamps for deterministic tests.
- Flush, writer-based flush, overflow flush helpers, and async capture helpers.

## Snippet

The buildable snippet in [examples/snippets/metrics/src/main.rs](../../examples/snippets/metrics/src/main.rs) records a
count metric, a high-resolution latency metric, a route dimension, metadata, and a deterministic timestamp.

Run it locally with:

```sh
cargo run -p metrics-snippet
```

Use `Metrics::flush` in Lambda handlers to emit to stdout. Use `Metrics::write_to` or
`Metrics::write_to_with_timestamp` in tests when you want to assert the rendered EMF line.
