//! Per-entry structured log builder.

use std::io::{self, Write};

use crate::{
    LogBuffer, LogBufferError, LogFields, LogFormatter, LogLevel, LogRedactor, LogValue, Logger,
    normalize_key,
};

/// A structured log entry being prepared for rendering or emission.
#[derive(Clone, Debug)]
pub struct LogEntry<'logger> {
    logger: &'logger Logger,
    level: LogLevel,
    message: String,
    fields: LogFields,
    event: Option<LogValue>,
}

impl<'logger> LogEntry<'logger> {
    pub(crate) fn new(
        logger: &'logger Logger,
        level: LogLevel,
        message: impl Into<String>,
    ) -> Self {
        Self {
            logger,
            level,
            message: message.into(),
            fields: LogFields::new(),
            event: None,
        }
    }

    /// Adds a temporary field to this log entry.
    ///
    /// Temporary fields apply only to this entry. Blank field names are ignored.
    #[must_use]
    pub fn field(mut self, key: impl Into<String>, value: impl Into<LogValue>) -> Self {
        if let Some(key) = normalize_key(key) {
            self.fields.insert(key, value.into());
        }
        self
    }

    /// Adds multiple temporary fields to this log entry.
    ///
    /// Temporary fields apply only to this entry. Blank field names are ignored.
    #[must_use]
    pub fn fields<I, K, V>(mut self, fields: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<LogValue>,
    {
        for (key, value) in fields {
            if let Some(key) = normalize_key(key) {
                self.fields.insert(key, value.into());
            }
        }
        self
    }

    /// Attaches an incoming Lambda event to this entry.
    ///
    /// The event is rendered under the `event` key only when event logging is
    /// enabled on the logger configuration.
    #[must_use]
    pub fn event(mut self, event: impl Into<LogValue>) -> Self {
        self.event = Some(event.into());
        self
    }

    /// Returns the severity level for this entry.
    #[must_use]
    pub fn level(&self) -> LogLevel {
        self.level
    }

    /// Returns the human-readable message for this entry.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Returns the temporary fields attached to this entry.
    #[must_use]
    pub fn fields_ref(&self) -> &LogFields {
        &self.fields
    }

    /// Returns the event attached to this entry, if any.
    #[must_use]
    pub fn event_ref(&self) -> Option<&LogValue> {
        self.event.as_ref()
    }

    /// Renders this entry as a JSON log line when it meets the logger level.
    ///
    /// The returned string does not include a trailing newline, so callers can
    /// write it with `println!` for Lambda stdout.
    #[must_use]
    pub fn render(&self) -> Option<String> {
        self.logger.render_entry(self)
    }

    /// Renders this entry with a custom formatter when it meets the logger level.
    #[must_use]
    pub fn render_with_formatter(&self, formatter: &impl LogFormatter) -> Option<String> {
        self.logger.render_entry_with_formatter(self, formatter)
    }

    /// Renders this entry with a custom redaction hook before JSON formatting.
    #[must_use]
    pub fn render_with_redactor(&self, redactor: &impl LogRedactor) -> Option<String> {
        self.logger.render_entry_with_redactor(self, redactor)
    }

    /// Renders this entry with a custom redaction hook and formatter.
    #[must_use]
    pub fn render_with_hooks(
        &self,
        redactor: &impl LogRedactor,
        formatter: &impl LogFormatter,
    ) -> Option<String> {
        self.logger
            .render_entry_with_hooks(self, redactor, formatter)
    }

    /// Emits this entry to stdout when it meets the logger level.
    ///
    /// Returns whether a line was emitted.
    pub fn emit(&self) -> bool {
        let mut stdout = io::stdout().lock();
        self.write_to(&mut stdout).unwrap_or(false)
    }

    /// Writes this entry to a stream when it meets the logger level.
    ///
    /// Returns whether a line was written. The written line includes a trailing
    /// newline.
    ///
    /// # Errors
    ///
    /// Returns any error reported by the provided writer.
    pub fn write_to(&self, writer: &mut impl Write) -> io::Result<bool> {
        if let Some(line) = self.render() {
            writeln!(writer, "{line}")?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Buffers this entry under a request, trace, or invocation key.
    ///
    /// Returns `Ok(false)` when the entry is filtered by the logger level or is
    /// more severe than the configured buffer verbosity.
    ///
    /// # Errors
    ///
    /// Returns [`LogBufferError`] when the rendered line is larger than the
    /// configured per-key buffer capacity.
    pub fn buffer_to(
        &self,
        buffer: &mut LogBuffer,
        key: impl Into<String>,
    ) -> Result<bool, LogBufferError> {
        buffer.record(key, self)
    }
}
