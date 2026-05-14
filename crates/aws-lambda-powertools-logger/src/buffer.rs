//! Request-keyed logger buffer.

use std::{
    collections::{BTreeMap, VecDeque},
    error::Error,
    fmt,
};

use crate::{LogEntry, LogLevel};

const DEFAULT_MAX_BYTES: usize = 20 * 1024;

/// Default key used when a blank buffer key is provided.
pub const DEFAULT_LOG_BUFFER_KEY: &str = "default";

/// Configuration for bounded log buffering.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LogBufferConfig {
    max_bytes: usize,
    buffer_at_verbosity: LogLevel,
    flush_on_error_log: bool,
}

impl LogBufferConfig {
    /// Creates a log buffer configuration with Powertools-compatible defaults.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            max_bytes: DEFAULT_MAX_BYTES,
            buffer_at_verbosity: LogLevel::Debug,
            flush_on_error_log: true,
        }
    }

    /// Returns a copy with the maximum bytes stored per key.
    ///
    /// A zero value is clamped to one byte so the buffer always has a positive
    /// capacity.
    #[must_use]
    pub fn with_max_bytes(mut self, max_bytes: usize) -> Self {
        self.max_bytes = max_bytes.max(1);
        self
    }

    /// Returns a copy with the highest verbosity level that should be buffered.
    ///
    /// Levels less severe than or equal to this value are buffered. More severe
    /// levels can be emitted immediately by caller code.
    #[must_use]
    pub const fn with_buffer_at_verbosity(mut self, level: LogLevel) -> Self {
        self.buffer_at_verbosity = level;
        self
    }

    /// Returns a copy with automatic error-log flush behavior enabled or disabled.
    #[must_use]
    pub const fn with_flush_on_error_log(mut self, enabled: bool) -> Self {
        self.flush_on_error_log = enabled;
        self
    }

    /// Returns the maximum bytes stored per key.
    #[must_use]
    pub const fn max_bytes(&self) -> usize {
        self.max_bytes
    }

    /// Returns the highest verbosity level that should be buffered.
    #[must_use]
    pub const fn buffer_at_verbosity(&self) -> LogLevel {
        self.buffer_at_verbosity
    }

    /// Returns whether caller code should flush the buffer before error logs.
    #[must_use]
    pub const fn flush_on_error_log(&self) -> bool {
        self.flush_on_error_log
    }

    /// Returns whether an entry at `level` should be buffered.
    #[must_use]
    pub fn buffers_level(&self, level: LogLevel) -> bool {
        level <= self.buffer_at_verbosity
    }
}

impl Default for LogBufferConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Error returned when a log line cannot fit in a buffer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LogBufferError {
    /// A rendered log line is larger than the configured per-key buffer.
    LineTooLarge {
        /// Configured maximum bytes per key.
        max_bytes: usize,
        /// Rendered log line size in bytes.
        line_bytes: usize,
    },
}

impl fmt::Display for LogBufferError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LineTooLarge {
                max_bytes,
                line_bytes,
            } => write!(
                formatter,
                "log line is {line_bytes} bytes but buffer capacity is {max_bytes} bytes"
            ),
        }
    }
}

impl Error for LogBufferError {}

/// Bounded log buffer keyed by request, trace, or invocation identifiers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LogBuffer {
    config: LogBufferConfig,
    groups: BTreeMap<String, BufferedGroup>,
}

impl LogBuffer {
    /// Creates a log buffer with explicit configuration.
    #[must_use]
    pub fn new(config: LogBufferConfig) -> Self {
        Self {
            config,
            groups: BTreeMap::new(),
        }
    }

    /// Returns the buffer configuration.
    #[must_use]
    pub const fn config(&self) -> &LogBufferConfig {
        &self.config
    }

    /// Returns whether an entry at `level` should be buffered.
    #[must_use]
    pub fn should_buffer(&self, level: LogLevel) -> bool {
        self.config.buffers_level(level)
    }

    /// Records a rendered log entry under a key.
    ///
    /// Returns `Ok(false)` when the entry is filtered by the logger level or is
    /// more severe than the configured buffer verbosity.
    ///
    /// # Errors
    ///
    /// Returns [`LogBufferError`] when the rendered line is larger than the
    /// configured per-key capacity.
    pub fn record(
        &mut self,
        key: impl Into<String>,
        entry: &LogEntry<'_>,
    ) -> Result<bool, LogBufferError> {
        if !self.should_buffer(entry.level()) {
            return Ok(false);
        }

        if let Some(line) = entry.render() {
            self.push_line(key, line)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Adds an already-rendered log line under a key.
    ///
    /// Oldest lines for the same key are evicted until the new line fits.
    ///
    /// # Errors
    ///
    /// Returns [`LogBufferError`] when `line` is larger than the configured
    /// per-key capacity.
    pub fn push_line(
        &mut self,
        key: impl Into<String>,
        line: impl Into<String>,
    ) -> Result<(), LogBufferError> {
        let line = line.into();
        let line_bytes = line.len();
        if line_bytes > self.config.max_bytes {
            return Err(LogBufferError::LineTooLarge {
                max_bytes: self.config.max_bytes,
                line_bytes,
            });
        }

        let group = self.groups.entry(normalize_key(key)).or_default();
        group.push(line, line_bytes, self.config.max_bytes);
        Ok(())
    }

    /// Flushes and removes buffered lines for `key`.
    #[must_use]
    pub fn flush(&mut self, key: impl Into<String>) -> Vec<String> {
        self.groups
            .remove(&normalize_key(key))
            .map_or_else(Vec::new, BufferedGroup::into_lines)
    }

    /// Flushes and removes buffered lines for all keys.
    #[must_use]
    pub fn flush_all(&mut self) -> BTreeMap<String, Vec<String>> {
        std::mem::take(&mut self.groups)
            .into_iter()
            .map(|(key, group)| (key, group.into_lines()))
            .collect()
    }

    /// Clears buffered lines for `key`.
    pub fn clear(&mut self, key: impl Into<String>) {
        self.groups.remove(&normalize_key(key));
    }

    /// Clears all buffered lines.
    pub fn clear_all(&mut self) {
        self.groups.clear();
    }

    /// Returns the number of buffered lines for `key`.
    #[must_use]
    pub fn len(&self, key: impl Into<String>) -> usize {
        self.groups
            .get(&normalize_key(key))
            .map_or(0, |group| group.lines.len())
    }

    /// Returns whether `key` has no buffered lines.
    #[must_use]
    pub fn is_empty(&self, key: impl Into<String>) -> bool {
        self.len(key) == 0
    }

    /// Returns the current buffered byte size for `key`.
    #[must_use]
    pub fn current_bytes(&self, key: impl Into<String>) -> usize {
        self.groups
            .get(&normalize_key(key))
            .map_or(0, |group| group.bytes)
    }

    /// Returns whether any line has been evicted for `key`.
    #[must_use]
    pub fn has_evicted(&self, key: impl Into<String>) -> bool {
        self.groups
            .get(&normalize_key(key))
            .is_some_and(|group| group.evicted)
    }
}

impl Default for LogBuffer {
    fn default() -> Self {
        Self::new(LogBufferConfig::new())
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct BufferedGroup {
    lines: VecDeque<BufferedLine>,
    bytes: usize,
    evicted: bool,
}

impl BufferedGroup {
    fn push(&mut self, line: String, line_bytes: usize, max_bytes: usize) {
        while self.bytes + line_bytes > max_bytes {
            if let Some(removed) = self.lines.pop_front() {
                self.bytes -= removed.bytes;
                self.evicted = true;
            } else {
                break;
            }
        }

        self.bytes += line_bytes;
        self.lines.push_back(BufferedLine {
            line,
            bytes: line_bytes,
        });
    }

    fn into_lines(self) -> Vec<String> {
        self.lines.into_iter().map(|line| line.line).collect()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct BufferedLine {
    line: String,
    bytes: usize,
}

fn normalize_key(key: impl Into<String>) -> String {
    let key = key.into();
    let key = key.trim();
    if key.is_empty() {
        DEFAULT_LOG_BUFFER_KEY.to_owned()
    } else {
        key.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::{DEFAULT_LOG_BUFFER_KEY, LogBuffer, LogBufferConfig, LogBufferError, LogLevel};
    use crate::{Logger, LoggerConfig};

    #[test]
    fn default_config_buffers_debug_and_more_verbose_logs() {
        let config = LogBufferConfig::new();

        assert_eq!(config.max_bytes(), 20 * 1024);
        assert_eq!(config.buffer_at_verbosity(), LogLevel::Debug);
        assert!(config.flush_on_error_log());
        assert!(config.buffers_level(LogLevel::Trace));
        assert!(config.buffers_level(LogLevel::Debug));
        assert!(!config.buffers_level(LogLevel::Info));
    }

    #[test]
    fn records_rendered_entries_by_key_and_flushes_fifo() {
        let logger = Logger::with_config(LoggerConfig::new("orders").with_level(LogLevel::Debug));
        let mut buffer = LogBuffer::new(
            LogBufferConfig::new()
                .with_max_bytes(1024)
                .with_buffer_at_verbosity(LogLevel::Info),
        );

        assert!(
            logger
                .debug("loaded")
                .field("order_id", "order-1")
                .buffer_to(&mut buffer, "trace-1")
                .expect("buffered log")
        );
        assert!(
            logger
                .info("accepted")
                .buffer_to(&mut buffer, "trace-1")
                .expect("buffered log")
        );

        assert_eq!(buffer.len("trace-1"), 2);
        assert_eq!(
            buffer.flush("trace-1"),
            vec![
                "{\"level\":\"DEBUG\",\"message\":\"loaded\",\"order_id\":\"order-1\",\
                 \"service\":\"orders\"}"
                    .replace(['\n', ' '], ""),
                "{\"level\":\"INFO\",\"message\":\"accepted\",\"service\":\"orders\"}".to_owned(),
            ]
        );
        assert!(buffer.is_empty("trace-1"));
    }

    #[test]
    fn skips_entries_filtered_by_logger_level() {
        let logger = Logger::with_config(LoggerConfig::new("orders").with_level(LogLevel::Info));
        let mut buffer = LogBuffer::default();

        assert!(
            !logger
                .debug("details")
                .buffer_to(&mut buffer, "trace-1")
                .expect("filtered log")
        );
        assert!(buffer.is_empty("trace-1"));
    }

    #[test]
    fn skips_entries_above_buffer_verbosity() {
        let logger = Logger::with_config(LoggerConfig::new("orders").with_level(LogLevel::Debug));
        let mut buffer =
            LogBuffer::new(LogBufferConfig::new().with_buffer_at_verbosity(LogLevel::Debug));

        assert!(
            !logger
                .info("accepted")
                .buffer_to(&mut buffer, "trace-1")
                .expect("bypassed log")
        );
        assert!(buffer.is_empty("trace-1"));
    }

    #[test]
    fn evicts_oldest_lines_when_key_exceeds_capacity() {
        let mut buffer = LogBuffer::new(LogBufferConfig::new().with_max_bytes(10));

        buffer.push_line("trace-1", "12345").expect("first line");
        buffer.push_line("trace-1", "abcdef").expect("second line");

        assert_eq!(buffer.current_bytes("trace-1"), 6);
        assert!(buffer.has_evicted("trace-1"));
        assert_eq!(buffer.flush("trace-1"), vec!["abcdef".to_owned()]);
    }

    #[test]
    fn rejects_lines_larger_than_capacity() {
        let mut buffer = LogBuffer::new(LogBufferConfig::new().with_max_bytes(4));

        assert_eq!(
            buffer.push_line("trace-1", "12345"),
            Err(LogBufferError::LineTooLarge {
                max_bytes: 4,
                line_bytes: 5
            })
        );
        assert!(buffer.is_empty("trace-1"));
    }

    #[test]
    fn normalizes_blank_keys_and_flushes_all_groups() {
        let mut buffer = LogBuffer::default();

        buffer.push_line(" ", "default-line").expect("default line");
        buffer
            .push_line("trace-1", "trace-line")
            .expect("trace line");

        assert_eq!(buffer.len(DEFAULT_LOG_BUFFER_KEY), 1);
        let flushed = buffer.flush_all();

        assert_eq!(
            flushed.get(DEFAULT_LOG_BUFFER_KEY),
            Some(&vec!["default-line".to_owned()])
        );
        assert_eq!(flushed.get("trace-1"), Some(&vec!["trace-line".to_owned()]));
        assert!(buffer.is_empty(DEFAULT_LOG_BUFFER_KEY));
        assert!(buffer.is_empty("trace-1"));
    }
}
