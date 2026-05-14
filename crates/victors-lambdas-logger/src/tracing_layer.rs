//! `tracing` subscriber integration.

use std::{
    fmt,
    io::{self, Write},
    sync::Mutex,
};

use tracing::{
    Event, Level, Subscriber,
    field::{Field, Visit},
};
use tracing_subscriber::{Layer, layer::Context};

use crate::{LogFields, LogLevel, LogValue, Logger};

/// A `tracing-subscriber` layer that renders events through [`Logger`].
///
/// The layer preserves the logger's level filtering, persistent fields,
/// redaction, sampling behavior, and JSON rendering. Event fields recorded by
/// `tracing` are attached as temporary log fields.
pub struct LoggerLayer<W> {
    logger: Logger,
    writer: Mutex<W>,
}

impl LoggerLayer<io::Stdout> {
    /// Creates a layer that writes rendered log events to stdout.
    #[must_use]
    pub fn new(logger: Logger) -> Self {
        Self::with_writer(logger, io::stdout())
    }
}

impl<W> LoggerLayer<W>
where
    W: Write,
{
    /// Creates a layer that writes rendered log events to `writer`.
    #[must_use]
    pub fn with_writer(logger: Logger, writer: W) -> Self {
        Self {
            logger,
            writer: Mutex::new(writer),
        }
    }

    /// Returns the logger used by this layer.
    #[must_use]
    pub const fn logger(&self) -> &Logger {
        &self.logger
    }

    /// Consumes this layer and returns its writer.
    ///
    /// # Panics
    ///
    /// Panics when the writer mutex is poisoned.
    #[must_use]
    pub fn into_writer(self) -> W {
        self.writer
            .into_inner()
            .expect("logger layer writer mutex should not be poisoned")
    }
}

impl<S, W> Layer<S> for LoggerLayer<W>
where
    S: Subscriber,
    W: Write + Send + Sync + 'static,
{
    fn on_event(&self, event: &Event<'_>, _context: Context<'_, S>) {
        let mut visitor = EventVisitor::default();
        event.record(&mut visitor);

        let metadata = event.metadata();
        let level = log_level_from_tracing(*metadata.level());
        let message = visitor
            .message
            .unwrap_or_else(|| metadata.name().to_owned());
        let mut entry = self
            .logger
            .entry(level, message)
            .field("target", metadata.target());

        if let Some(module_path) = metadata.module_path() {
            entry = entry.field("module_path", module_path);
        }
        if let Some(file) = metadata.file() {
            entry = entry.field("file", file);
        }
        if let Some(line) = metadata.line() {
            entry = entry.field("line", u64::from(line));
        }
        for (key, value) in visitor.fields {
            entry = entry.field(key, value);
        }

        let Some(line) = entry.render() else {
            return;
        };
        if let Ok(mut writer) = self.writer.lock() {
            let _ = writeln!(writer, "{line}");
        }
    }
}

#[derive(Default)]
struct EventVisitor {
    message: Option<String>,
    fields: LogFields,
}

impl EventVisitor {
    fn record_value(&mut self, field: &Field, value: impl Into<LogValue>) {
        let value = value.into();

        if field.name() == "message" {
            self.message = Some(string_value(&value));
        } else {
            self.fields.insert(field.name().to_owned(), value);
        }
    }
}

impl Visit for EventVisitor {
    fn record_bool(&mut self, field: &Field, value: bool) {
        self.record_value(field, value);
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.record_value(field, value);
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.record_value(field, value);
    }

    fn record_i128(&mut self, field: &Field, value: i128) {
        self.record_value(field, value);
    }

    fn record_u128(&mut self, field: &Field, value: u128) {
        self.record_value(field, value);
    }

    fn record_f64(&mut self, field: &Field, value: f64) {
        self.record_value(field, value);
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.record_value(field, value);
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.record_value(field, format!("{value:?}"));
    }
}

fn log_level_from_tracing(level: Level) -> LogLevel {
    match level {
        Level::TRACE => LogLevel::Trace,
        Level::DEBUG => LogLevel::Debug,
        Level::INFO => LogLevel::Info,
        Level::WARN => LogLevel::Warn,
        Level::ERROR => LogLevel::Error,
    }
}

fn string_value(value: &LogValue) -> String {
    let rendered = value.to_json_string();
    if rendered.starts_with('"') && rendered.ends_with('"') && rendered.len() >= 2 {
        rendered[1..rendered.len() - 1].to_owned()
    } else {
        rendered
    }
}

#[cfg(test)]
mod tests {
    use std::{
        io::{self, Write},
        sync::{Arc, Mutex},
    };

    use tracing::subscriber::with_default;
    use tracing_subscriber::{Registry, layer::SubscriberExt};

    use super::LoggerLayer;
    use crate::{LogLevel, Logger, LoggerConfig};

    #[derive(Clone, Debug, Default)]
    struct SharedWriter {
        output: Arc<Mutex<Vec<u8>>>,
    }

    impl SharedWriter {
        fn output(&self) -> String {
            let bytes = self
                .output
                .lock()
                .expect("writer should not be poisoned")
                .clone();

            String::from_utf8(bytes).expect("log output should be utf-8")
        }
    }

    impl Write for SharedWriter {
        fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
            self.output
                .lock()
                .map_err(|_| io::Error::other("writer poisoned"))?
                .extend_from_slice(buffer);
            Ok(buffer.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn tracing_layer_renders_events_through_logger() {
        let writer = SharedWriter::default();
        let layer = LoggerLayer::with_writer(
            Logger::with_config(LoggerConfig::new("orders")).with_field("cold_start", true),
            writer.clone(),
        );
        let subscriber = Registry::default().with(layer);

        with_default(subscriber, || {
            tracing::info!(order_id = "order-1", quantity = 2_u64, "created");
        });

        let output = writer.output();

        assert!(output.ends_with('\n'));
        assert!(output.contains(r#""level":"INFO""#));
        assert!(output.contains(r#""message":"created""#));
        assert!(output.contains(r#""service":"orders""#));
        assert!(output.contains(r#""cold_start":true"#));
        assert!(output.contains(r#""order_id":"order-1""#));
        assert!(output.contains(r#""quantity":2"#));
        assert!(output.contains(r#""target":"#));
    }

    #[test]
    fn tracing_layer_respects_logger_level_filter() {
        let writer = SharedWriter::default();
        let layer = LoggerLayer::with_writer(
            Logger::with_config(LoggerConfig::new("orders").with_level(LogLevel::Warn)),
            writer.clone(),
        );
        let subscriber = Registry::default().with(layer);

        with_default(subscriber, || {
            tracing::info!("ignored");
            tracing::warn!("kept");
        });

        let output = writer.output();

        assert!(!output.contains("ignored"));
        assert!(output.contains(r#""message":"kept""#));
        assert!(output.contains(r#""level":"WARN""#));
    }
}
