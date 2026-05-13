# Logger

The logger utility renders structured JSON log entries and can emit them to stdout. It is exposed through the
`logger` Cargo feature on the umbrella crate:

```toml
aws-lambda-powertools = { version = "0.1", features = ["logger"] }
```

## Configuration

Use `LoggerConfig::new` for explicit service configuration or `LoggerConfig::from_env` to read Powertools-compatible
environment variables.

| Environment variable | Effect |
| --- | --- |
| `POWERTOOLS_SERVICE_NAME` | Sets the `service` field. |
| `POWERTOOLS_LOG_LEVEL` | Sets the minimum emitted level. Unknown values fall back to `INFO`. |
| `POWERTOOLS_LOGGER_LOG_EVENT` | Includes per-entry event payloads when truthy. |
| `POWERTOOLS_LOGGER_SAMPLE_RATE` | Enables debug log sampling for a fraction from `0.0` to `1.0`. |

## Supported Behavior

- Severity filtering with `TRACE`, `DEBUG`, `INFO`, `WARN`, and `ERROR`.
- Persistent fields with per-entry overrides.
- Optional event rendering.
- Debug sampling that can temporarily lower the effective threshold to `DEBUG`.
- Correlation ID and Lambda context fields.
- Recursive key redaction.
- Custom formatter and redaction hooks.
- Optional `tracing` subscriber integration through the `logger-tracing` feature.

## Snippet

The buildable snippet in [examples/snippets/logger/src/main.rs](../../examples/snippets/logger/src/main.rs) shows a
logger enriched with Lambda context, a correlation ID, a persistent component field, event rendering, and redaction.

Run it locally with:

```sh
cargo run -p logger-snippet
```

Use `Logger::emit` for normal Lambda stdout emission, or `LogEntry::render` when tests need to assert the JSON line
without writing to stdout.
