//! Logger type and log levels.

use std::collections::BTreeSet;
use std::time::{SystemTime, UNIX_EPOCH};

use victors_lambdas_core::env;

use crate::{
    JsonLogFormatter, LambdaLogContext, LogEntry, LogFields, LogFormatter, LogRedactor, LogValue,
    LoggerConfig, normalize_key,
};

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
        let mut source = env::var;
        Self::from_env_source(&mut source)
    }

    pub(crate) fn from_env_source(source: &mut impl FnMut(&str) -> Option<String>) -> Self {
        source(env::POWERTOOLS_LOG_LEVEL)
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
    redacted_fields: BTreeSet<String>,
    sampled: bool,
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
        let sampled = should_sample(config.sample_rate());
        Self {
            config,
            fields: LogFields::new(),
            redacted_fields: BTreeSet::new(),
            sampled,
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

    /// Returns the effective log level after debug sampling is applied.
    #[must_use]
    pub fn effective_level(&self) -> LogLevel {
        if self.sampled && self.config.level() > LogLevel::Debug {
            LogLevel::Debug
        } else {
            self.config.level()
        }
    }

    /// Returns the configured debug sampling rate.
    #[must_use]
    pub fn sample_rate(&self) -> f64 {
        self.config.sample_rate()
    }

    /// Returns whether this logger is currently sampled into debug logging.
    #[must_use]
    pub fn is_sampled(&self) -> bool {
        self.sampled
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

    /// Returns the field names that are redacted recursively before rendering.
    #[must_use]
    pub fn redacted_fields(&self) -> &BTreeSet<String> {
        &self.redacted_fields
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

    /// Returns a copy of the logger with a persistent correlation id.
    #[must_use]
    pub fn with_correlation_id(mut self, correlation_id: impl Into<String>) -> Self {
        self.set_correlation_id(correlation_id);
        self
    }

    /// Returns a copy of the logger enriched with Lambda context fields.
    #[must_use]
    pub fn with_lambda_context(mut self, context: &impl LambdaLogContext) -> Self {
        self.append_lambda_context(context);
        self
    }

    /// Returns a copy of the logger with a redacted field name.
    #[must_use]
    pub fn with_redacted_field(mut self, key: impl Into<String>) -> Self {
        self.redact_field(key);
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

    /// Sets a persistent correlation id.
    ///
    /// Passing a blank value removes the current correlation id.
    pub fn set_correlation_id(&mut self, correlation_id: impl Into<String>) -> &mut Self {
        if let Some(correlation_id) = normalize_key(correlation_id) {
            self.fields
                .insert("correlation_id".to_owned(), correlation_id.into());
        } else {
            self.fields.remove("correlation_id");
        }
        self
    }

    /// Returns the current persistent correlation id value.
    #[must_use]
    pub fn correlation_id(&self) -> Option<&LogValue> {
        self.fields.get("correlation_id")
    }

    /// Clears the persistent correlation id.
    pub fn clear_correlation_id(&mut self) -> Option<LogValue> {
        self.fields.remove("correlation_id")
    }

    /// Appends Lambda context fields to persistent log fields.
    pub fn append_lambda_context(&mut self, context: &impl LambdaLogContext) -> &mut Self {
        self.append_field("function_request_id", context.function_request_id());
        self.append_field("function_name", context.function_name());
        if let Some(function_version) = context.function_version() {
            self.append_field("function_version", function_version);
        }
        if let Some(function_arn) = context.function_arn() {
            self.append_field("function_arn", function_arn);
        }
        if let Some(function_memory_size) = context.function_memory_size() {
            self.append_field("function_memory_size", function_memory_size);
        }
        if let Some(cold_start) = context.cold_start() {
            self.append_field("cold_start", cold_start);
        }
        self
    }

    /// Redacts a field by name before rendering.
    ///
    /// Redaction is recursive, so matching keys inside objects and arrays are
    /// replaced with `"[REDACTED]"`.
    pub fn redact_field(&mut self, key: impl Into<String>) -> &mut Self {
        if let Some(key) = normalize_key(key) {
            self.redacted_fields.insert(key);
        }
        self
    }

    /// Redacts multiple field names before rendering.
    pub fn redact_fields<I, K>(&mut self, keys: I) -> &mut Self
    where
        I: IntoIterator<Item = K>,
        K: Into<String>,
    {
        for key in keys {
            self.redact_field(key);
        }
        self
    }

    /// Clears configured redacted field names.
    pub fn clear_redacted_fields(&mut self) {
        self.redacted_fields.clear();
    }

    /// Recomputes the debug sampling decision.
    pub fn refresh_sampling_decision(&mut self) -> &mut Self {
        self.sampled = should_sample(self.config.sample_rate());
        self
    }

    /// Overrides the current debug sampling decision.
    ///
    /// This is useful for deterministic tests and handler integrations that
    /// make the sampling decision externally.
    pub fn set_sampling_decision(&mut self, sampled: bool) -> &mut Self {
        self.sampled = sampled;
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
        level >= self.effective_level()
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

    /// Renders a log entry using a custom formatter when it meets the configured threshold.
    #[must_use]
    pub fn render_with_formatter(
        &self,
        level: LogLevel,
        message: impl Into<String>,
        formatter: &impl LogFormatter,
    ) -> Option<String> {
        self.entry(level, message).render_with_formatter(formatter)
    }

    /// Renders a log entry using a custom redaction hook before JSON formatting.
    #[must_use]
    pub fn render_with_redactor(
        &self,
        level: LogLevel,
        message: impl Into<String>,
        redactor: &impl LogRedactor,
    ) -> Option<String> {
        self.entry(level, message).render_with_redactor(redactor)
    }

    /// Emits a log entry to stdout when it meets the configured threshold.
    ///
    /// Returns whether a line was emitted.
    pub fn emit(&self, level: LogLevel, message: impl Into<String>) -> bool {
        self.entry(level, message).emit()
    }

    pub(crate) fn render_entry(&self, entry: &LogEntry<'_>) -> Option<String> {
        let value = self.entry_value(entry)?;
        Some(if self.config.pretty_print() {
            value.to_json_pretty_string()
        } else {
            JsonLogFormatter.format(&value)
        })
    }

    pub(crate) fn render_entry_with_formatter(
        &self,
        entry: &LogEntry<'_>,
        formatter: &impl LogFormatter,
    ) -> Option<String> {
        self.entry_value(entry)
            .map(|value| formatter.format(&value))
    }

    pub(crate) fn render_entry_with_redactor(
        &self,
        entry: &LogEntry<'_>,
        redactor: &impl LogRedactor,
    ) -> Option<String> {
        self.render_entry_with_hooks(entry, redactor, &JsonLogFormatter)
    }

    pub(crate) fn render_entry_with_hooks(
        &self,
        entry: &LogEntry<'_>,
        redactor: &impl LogRedactor,
        formatter: &impl LogFormatter,
    ) -> Option<String> {
        let mut value = self.entry_value(entry)?;
        redactor.redact(&mut value);
        Some(formatter.format(&value))
    }

    pub(crate) fn entry_value(&self, entry: &LogEntry<'_>) -> Option<LogValue> {
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
        if self.sample_rate() > 0.0 {
            fields.insert("sampling_rate".to_owned(), self.sample_rate().into());
        }

        if self.logs_events() {
            if let Some(event) = entry.event_ref() {
                fields.insert("event".to_owned(), event.clone());
            }
        }

        let mut value = LogValue::from(fields);
        if !self.redacted_fields.is_empty() {
            value.redact_keys(&self.redacted_fields, &LogValue::string("[REDACTED]"));
        }

        Some(value)
    }
}

impl Default for Logger {
    fn default() -> Self {
        Self::new()
    }
}

fn should_sample(sample_rate: f64) -> bool {
    if sample_rate <= 0.0 {
        false
    } else if sample_rate >= 1.0 {
        true
    } else {
        sample_draw() < sample_rate
    }
}

fn sample_draw() -> f64 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.subsec_nanos());

    f64::from(nanos % 1_000_000) / 1_000_000.0
}

#[cfg(test)]
mod tests {
    use crate::{LambdaContextFields, LogLevel, LogValue, Logger, LoggerConfig};

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
    fn renders_pretty_entry_when_configured() {
        let logger = Logger::with_config(LoggerConfig::new("orders").with_pretty_print(true));

        assert_eq!(
            logger.info("created").render(),
            Some(
                "{\n    \"level\": \"INFO\",\n    \"message\": \"created\",\n    \"service\": \"orders\"\n}"
                    .to_owned()
            )
        );
    }

    #[test]
    fn sampling_decision_enables_debug_logs_and_renders_rate() {
        let mut logger = Logger::with_config(
            LoggerConfig::new("orders")
                .with_level(LogLevel::Info)
                .with_sample_rate(0.5),
        );

        logger.set_sampling_decision(true);

        assert_eq!(logger.effective_level(), LogLevel::Debug);
        assert!(logger.is_enabled(LogLevel::Debug));
        assert!(!logger.is_enabled(LogLevel::Trace));
        assert_eq!(
            logger.debug("details").render(),
            Some(
                "{\"level\":\"DEBUG\",\"message\":\"details\",\
                 \"sampling_rate\":0.5,\"service\":\"orders\"}"
                    .replace(['\n', ' '], "")
            )
        );

        logger.set_sampling_decision(false);

        assert_eq!(logger.effective_level(), LogLevel::Info);
        assert_eq!(logger.debug("details").render(), None);
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
    fn correlation_id_and_lambda_context_are_rendered() {
        let context = LambdaContextFields::new("req-1", "orders-fn")
            .with_function_version("$LATEST")
            .with_function_arn("arn:aws:lambda:us-east-1:123456789012:function:orders-fn")
            .with_function_memory_size(128)
            .with_cold_start(true);
        let mut logger = Logger::with_config(LoggerConfig::new("orders"))
            .with_correlation_id("corr-1")
            .with_lambda_context(&context);

        assert_eq!(logger.correlation_id(), Some(&LogValue::from("corr-1")));
        assert_eq!(
            logger.info("created").render(),
            Some(
                "{\"cold_start\":true,\"correlation_id\":\"corr-1\",\
                 \"function_arn\":\"arn:aws:lambda:us-east-1:123456789012:function:orders-fn\",\
                 \"function_memory_size\":128,\"function_name\":\"orders-fn\",\
                 \"function_request_id\":\"req-1\",\"function_version\":\"$LATEST\",\
                 \"level\":\"INFO\",\"message\":\"created\",\"service\":\"orders\"}"
                    .replace(['\n', ' '], "")
            )
        );

        assert_eq!(
            logger.clear_correlation_id(),
            Some(LogValue::from("corr-1"))
        );
        assert_eq!(logger.correlation_id(), None);
    }

    #[test]
    fn redacted_fields_are_replaced_recursively() {
        let logger = Logger::with_config(LoggerConfig::new("orders").with_event_logging(true))
            .with_redacted_field("password");

        assert_eq!(
            logger
                .info("created")
                .field("password", "top-secret")
                .event(LogValue::object([(
                    "user",
                    LogValue::object([
                        ("id", LogValue::from("user-1")),
                        ("password", LogValue::from("nested-secret")),
                    ]),
                )]))
                .render(),
            Some(
                "{\"event\":{\"user\":{\"id\":\"user-1\",\
                 \"password\":\"[REDACTED]\"}},\"level\":\"INFO\",\
                 \"message\":\"created\",\"password\":\"[REDACTED]\",\
                 \"service\":\"orders\"}"
                    .replace(['\n', ' '], "")
            )
        );
    }

    #[test]
    fn custom_redactor_hook_runs_after_configured_redaction() {
        let logger =
            Logger::with_config(LoggerConfig::new("orders")).with_redacted_field("password");

        assert_eq!(
            logger
                .info("created")
                .field("password", "secret")
                .field("token", "token-1")
                .render_with_redactor(&|value: &mut LogValue| {
                    value.redact_fields_with(["token"], "[MASKED]");
                }),
            Some(
                "{\"level\":\"INFO\",\"message\":\"created\",\
                 \"password\":\"[REDACTED]\",\"service\":\"orders\",\
                 \"token\":\"[MASKED]\"}"
                    .replace(['\n', ' '], "")
            )
        );
    }

    #[test]
    fn custom_formatter_can_render_structured_value() {
        let logger = Logger::with_config(LoggerConfig::new("orders"));

        assert_eq!(
            logger
                .info("created")
                .render_with_formatter(&|value: &LogValue| format!(
                    "custom:{}",
                    value.to_json_string()
                )),
            Some(
                "custom:{\"level\":\"INFO\",\"message\":\"created\",\"service\":\"orders\"}"
                    .to_owned()
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
