//! Logger type and log levels.

use aws_lambda_powertools_core::env;

use crate::{LogEntry, LogFields, LogValue, LoggerConfig, normalize_key};

/// Log severity level.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum LogLevel {
    /// Diagnostic trace logging.
    Trace,
    /// Diagnostic debug logging.
    Debug,
    /// Informational logging.
    Info,
    /// Warning logging.
    Warn,
    /// Error logging.
    Error,
}

impl LogLevel {
    /// Returns the uppercase log level name used in rendered entries.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Trace => "TRACE",
            Self::Debug => "DEBUG",
            Self::Info => "INFO",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
        }
    }

    /// Reads the log level from `POWERTOOLS_LOG_LEVEL`.
    #[must_use]
    pub fn from_env() -> Self {
        env::var(env::POWERTOOLS_LOG_LEVEL)
            .as_deref()
            .map_or(Self::Info, Self::from_name)
    }

    /// Parses a log level name.
    #[must_use]
    pub fn from_name(name: &str) -> Self {
        match name.trim().to_ascii_uppercase().as_str() {
            "TRACE" => Self::Trace,
            "DEBUG" => Self::Debug,
            "WARN" | "WARNING" => Self::Warn,
            "ERROR" => Self::Error,
            _ => Self::Info,
        }
    }
}

/// Structured logger facade.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Logger {
    config: LoggerConfig,
    fields: LogFields,
}

impl Logger {
    /// Creates a logger from environment configuration.
    #[must_use]
    pub fn new() -> Self {
        Self::with_config(LoggerConfig::from_env())
    }

    /// Creates a logger with explicit configuration.
    #[must_use]
    pub fn with_config(config: LoggerConfig) -> Self {
        Self {
            config,
            fields: LogFields::new(),
        }
    }

    /// Returns the logger configuration.
    #[must_use]
    pub fn config(&self) -> &LoggerConfig {
        &self.config
    }

    /// Returns the configured service name.
    #[must_use]
    pub fn service_name(&self) -> &str {
        self.config.service().service_name()
    }

    /// Returns the configured log level.
    #[must_use]
    pub fn level(&self) -> LogLevel {
        self.config.level()
    }

    /// Returns whether incoming events should be logged.
    #[must_use]
    pub fn logs_events(&self) -> bool {
        self.config.log_event()
    }

    /// Returns the persistent fields included in every rendered log entry.
    #[must_use]
    pub fn persistent_fields(&self) -> &LogFields {
        &self.fields
    }

    /// Returns a copy of the logger with an additional persistent field.
    ///
    /// Persistent fields are included in every rendered log entry. Blank field
    /// names are ignored.
    #[must_use]
    pub fn with_field(mut self, key: impl Into<String>, value: impl Into<LogValue>) -> Self {
        self.append_field(key, value);
        self
    }

    /// Returns a copy of the logger with additional persistent fields.
    ///
    /// Persistent fields are included in every rendered log entry. Blank field
    /// names are ignored.
    #[must_use]
    pub fn with_fields<I, K, V>(mut self, fields: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<LogValue>,
    {
        self.append_fields(fields);
        self
    }

    /// Adds a persistent field to the logger.
    ///
    /// Blank field names are ignored.
    pub fn append_field(
        &mut self,
        key: impl Into<String>,
        value: impl Into<LogValue>,
    ) -> &mut Self {
        if let Some(key) = normalize_key(key) {
            self.fields.insert(key, value.into());
        }
        self
    }

    /// Adds persistent fields to the logger.
    ///
    /// Blank field names are ignored.
    pub fn append_fields<I, K, V>(&mut self, fields: I) -> &mut Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<LogValue>,
    {
        for (key, value) in fields {
            self.append_field(key, value);
        }
        self
    }

    /// Removes a persistent field from the logger.
    pub fn remove_field(&mut self, key: &str) -> Option<LogValue> {
        self.fields.remove(key)
    }

    /// Clears all persistent fields from the logger.
    pub fn clear_fields(&mut self) {
        self.fields.clear();
    }

    /// Returns whether entries at `level` are enabled by the configured threshold.
    #[must_use]
    pub fn is_enabled(&self, level: LogLevel) -> bool {
        level >= self.level()
    }

    /// Creates a structured log entry at the provided severity level.
    #[must_use]
    pub fn entry(&self, level: LogLevel, message: impl Into<String>) -> LogEntry<'_> {
        LogEntry::new(self, level, message)
    }

    /// Creates a trace log entry.
    #[must_use]
    pub fn trace(&self, message: impl Into<String>) -> LogEntry<'_> {
        self.entry(LogLevel::Trace, message)
    }

    /// Creates a debug log entry.
    #[must_use]
    pub fn debug(&self, message: impl Into<String>) -> LogEntry<'_> {
        self.entry(LogLevel::Debug, message)
    }

    /// Creates an info log entry.
    #[must_use]
    pub fn info(&self, message: impl Into<String>) -> LogEntry<'_> {
        self.entry(LogLevel::Info, message)
    }

    /// Creates a warning log entry.
    #[must_use]
    pub fn warn(&self, message: impl Into<String>) -> LogEntry<'_> {
        self.entry(LogLevel::Warn, message)
    }

    /// Creates an error log entry.
    #[must_use]
    pub fn error(&self, message: impl Into<String>) -> LogEntry<'_> {
        self.entry(LogLevel::Error, message)
    }

    /// Renders a log entry as JSON when it meets the configured threshold.
    #[must_use]
    pub fn render(&self, level: LogLevel, message: impl Into<String>) -> Option<String> {
        self.entry(level, message).render()
    }

    /// Emits a log entry to stdout when it meets the configured threshold.
    ///
    /// Returns whether a line was emitted.
    pub fn emit(&self, level: LogLevel, message: impl Into<String>) -> bool {
        self.entry(level, message).emit()
    }

    pub(crate) fn render_entry(&self, entry: &LogEntry<'_>) -> Option<String> {
        if !self.is_enabled(entry.level()) {
            return None;
        }

        let mut fields = self.fields.clone();
        fields.extend(
            entry
                .fields_ref()
                .iter()
                .map(|(key, value)| (key.clone(), value.clone())),
        );

        fields.insert("level".to_owned(), entry.level().as_str().into());
        fields.insert("message".to_owned(), entry.message().into());
        fields.insert("service".to_owned(), self.service_name().into());

        if self.logs_events() {
            if let Some(event) = entry.event_ref() {
                fields.insert("event".to_owned(), event.clone());
            }
        }

        Some(LogValue::from(fields).to_json_string())
    }
}

impl Default for Logger {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use crate::{LogLevel, LogValue, Logger, LoggerConfig};

    #[test]
    fn log_levels_parse_names_and_filter_by_threshold() {
        let logger = Logger::with_config(LoggerConfig::new("orders").with_level(LogLevel::Warn));

        assert_eq!(LogLevel::from_name("warning"), LogLevel::Warn);
        assert_eq!(LogLevel::from_name("unknown"), LogLevel::Info);
        assert!(!logger.is_enabled(LogLevel::Info));
        assert!(logger.is_enabled(LogLevel::Warn));
        assert!(logger.is_enabled(LogLevel::Error));
    }

    #[test]
    fn renders_base_entry_as_json() {
        let logger = Logger::with_config(LoggerConfig::new("orders"));

        assert_eq!(
            logger.info("created").render(),
            Some("{\"level\":\"INFO\",\"message\":\"created\",\"service\":\"orders\"}".to_owned())
        );
    }

    #[test]
    fn persistent_and_temporary_fields_are_rendered() {
        let mut logger = Logger::with_config(LoggerConfig::new("orders"))
            .with_field("cold_start", true)
            .with_field("message", "ignored");
        logger.append_field("request_id", "abc-123");

        assert_eq!(
            logger
                .info("created")
                .field("request_id", "override")
                .field("attempt", 2)
                .field("  ", "ignored")
                .render(),
            Some(
                "{\"attempt\":2,\"cold_start\":true,\"level\":\"INFO\",\
                 \"message\":\"created\",\"request_id\":\"override\",\
                 \"service\":\"orders\"}"
                    .replace(['\n', ' '], "")
            )
        );
    }

    #[test]
    fn event_is_rendered_only_when_enabled() {
        let disabled = Logger::with_config(LoggerConfig::new("orders"));
        let enabled = Logger::with_config(
            LoggerConfig::new("orders")
                .with_level(LogLevel::Info)
                .with_event_logging(true),
        );

        assert_eq!(
            disabled
                .info("received")
                .event(LogValue::object([("id", "evt-1")]))
                .render(),
            Some("{\"level\":\"INFO\",\"message\":\"received\",\"service\":\"orders\"}".to_owned())
        );
        assert_eq!(
            enabled
                .info("received")
                .event(LogValue::object([("id", "evt-1")]))
                .render(),
            Some(
                "{\"event\":{\"id\":\"evt-1\"},\"level\":\"INFO\",\
                 \"message\":\"received\",\"service\":\"orders\"}"
                    .replace(['\n', ' '], "")
            )
        );
    }

    #[test]
    fn filtered_entries_do_not_render_or_write() {
        let logger = Logger::with_config(LoggerConfig::new("orders").with_level(LogLevel::Error));
        let mut output = Vec::new();

        assert_eq!(logger.warn("skipped").render(), None);
        assert!(
            !logger
                .warn("skipped")
                .write_to(&mut output)
                .expect("buffer writes should succeed")
        );
        assert!(output.is_empty());
    }

    #[test]
    fn writes_rendered_entries_with_newline() {
        let logger = Logger::with_config(LoggerConfig::new("orders"));
        let mut output = Vec::new();

        assert!(
            logger
                .error("failed")
                .write_to(&mut output)
                .expect("buffer writes should succeed")
        );

        assert_eq!(
            String::from_utf8(output).expect("logger output should be utf-8"),
            "{\"level\":\"ERROR\",\"message\":\"failed\",\"service\":\"orders\"}\n"
        );
    }

    #[test]
    fn persistent_fields_can_be_removed_or_cleared() {
        let mut logger = Logger::with_config(LoggerConfig::new("orders"))
            .with_fields([("request_id", "abc-123"), ("tenant", "north")]);

        assert_eq!(logger.persistent_fields().len(), 2);
        assert_eq!(logger.remove_field("tenant"), Some("north".into()));
        logger.clear_fields();

        assert!(logger.persistent_fields().is_empty());
    }
}
