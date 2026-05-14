//! AWS `AppSync` GraphQL scalar helpers.

use std::fmt;

use chrono::{Duration, Utc};
use uuid::Uuid;

const MIN_OFFSET_MINUTES: i16 = -12 * 60;
const MAX_OFFSET_MINUTES: i16 = 14 * 60;

/// Error returned by AWS `AppSync` scalar helpers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppSyncScalarError {
    /// The timezone offset is outside the range supported by AWS `AppSync` scalars.
    TimezoneOffsetOutOfRange {
        /// Offset from UTC in minutes.
        minutes: i16,
    },
    /// A random GraphQL `ID` value could not be generated.
    RandomIdUnavailable,
}

impl fmt::Display for AppSyncScalarError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TimezoneOffsetOutOfRange { minutes } => write!(
                formatter,
                "timezone offset must be between -720 and 840 minutes inclusive, got {minutes}"
            ),
            Self::RandomIdUnavailable => {
                formatter.write_str("failed to generate random AppSync ID")
            }
        }
    }
}

impl std::error::Error for AppSyncScalarError {}

/// Result returned by AWS `AppSync` scalar helpers.
pub type AppSyncScalarResult<T> = Result<T, AppSyncScalarError>;

/// Timezone offset used when formatting AWS `AppSync` date and time scalars.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AppSyncTimeOffset {
    minutes: i16,
}

impl AppSyncTimeOffset {
    /// UTC offset.
    pub const UTC: Self = Self { minutes: 0 };

    /// Creates an offset from whole hours.
    ///
    /// # Errors
    ///
    /// Returns [`AppSyncScalarError`] when the offset is outside the inclusive
    /// `-12:00` to `+14:00` range.
    pub fn from_hours(hours: i8) -> AppSyncScalarResult<Self> {
        Self::from_minutes(i16::from(hours) * 60)
    }

    /// Creates an offset from minutes.
    ///
    /// # Errors
    ///
    /// Returns [`AppSyncScalarError`] when the offset is outside the inclusive
    /// `-12:00` to `+14:00` range.
    pub const fn from_minutes(minutes: i16) -> AppSyncScalarResult<Self> {
        if minutes < MIN_OFFSET_MINUTES || minutes > MAX_OFFSET_MINUTES {
            return Err(AppSyncScalarError::TimezoneOffsetOutOfRange { minutes });
        }

        Ok(Self { minutes })
    }

    /// Returns the offset from UTC in minutes.
    #[must_use]
    pub const fn minutes(self) -> i16 {
        self.minutes
    }
}

/// Generates a unique GraphQL `ID` scalar value.
///
/// # Errors
///
/// Returns [`AppSyncScalarError`] when the operating system random number
/// generator is unavailable.
pub fn make_id() -> AppSyncScalarResult<String> {
    let mut bytes = [0_u8; 16];
    getrandom::getrandom(&mut bytes).map_err(|_| AppSyncScalarError::RandomIdUnavailable)?;
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;

    Ok(Uuid::from_bytes(bytes).to_string())
}

/// Generates an AWS `AppSync` `AWSTimestamp` value.
#[must_use]
pub fn aws_timestamp() -> i64 {
    Utc::now().timestamp()
}

/// Generates an AWS `AppSync` `AWSDate` value for the provided timezone offset.
#[must_use]
pub fn aws_date(offset: AppSyncTimeOffset) -> String {
    formatted_time(offset, "%Y-%m-%d")
}

/// Generates an AWS `AppSync` `AWSTime` value for the provided timezone offset.
#[must_use]
pub fn aws_time(offset: AppSyncTimeOffset) -> String {
    formatted_time(offset, "%H:%M:%S%.3f")
}

/// Generates an AWS `AppSync` `AWSDateTime` value for the provided timezone offset.
#[must_use]
pub fn aws_date_time(offset: AppSyncTimeOffset) -> String {
    formatted_time(offset, "%Y-%m-%dT%H:%M:%S%.3f")
}

fn formatted_time(offset: AppSyncTimeOffset, format: &str) -> String {
    let adjusted = Utc::now() + Duration::minutes(i64::from(offset.minutes()));
    format!("{}{}", adjusted.format(format), offset_suffix(offset))
}

fn offset_suffix(offset: AppSyncTimeOffset) -> String {
    let minutes = offset.minutes();
    if minutes == 0 {
        return "Z".to_owned();
    }

    let sign = if minutes.is_positive() { '+' } else { '-' };
    let minutes = minutes.unsigned_abs();
    let hours = minutes / 60;
    let minutes = minutes % 60;

    format!("{sign}{hours:02}:{minutes:02}:00")
}

#[cfg(test)]
mod tests {
    use super::{
        AppSyncScalarError, AppSyncTimeOffset, aws_date, aws_date_time, aws_time, aws_timestamp,
        make_id,
    };

    #[test]
    fn creates_appsync_time_offsets() {
        let offset = AppSyncTimeOffset::from_minutes(330).expect("offset should be valid");
        let west = AppSyncTimeOffset::from_minutes(-720).expect("offset should be valid");
        let east = AppSyncTimeOffset::from_hours(14).expect("offset should be valid");

        assert_eq!(offset.minutes(), 330);
        assert_eq!(AppSyncTimeOffset::from_hours(-12), Ok(west));
        assert_eq!(east.minutes(), 840);
    }

    #[test]
    fn rejects_appsync_time_offsets_outside_supported_range() {
        let error = AppSyncTimeOffset::from_minutes(841).expect_err("offset should be invalid");

        assert_eq!(
            error,
            AppSyncScalarError::TimezoneOffsetOutOfRange { minutes: 841 }
        );
    }

    #[test]
    fn formats_appsync_scalar_values() {
        let utc = AppSyncTimeOffset::UTC;
        let east = AppSyncTimeOffset::from_minutes(330).expect("offset should be valid");
        let west = AppSyncTimeOffset::from_minutes(-570).expect("offset should be valid");

        assert!(aws_date(utc).ends_with('Z'));
        assert!(aws_time(east).ends_with("+05:30:00"));
        assert!(aws_time(west).ends_with("-09:30:00"));
        assert!(aws_date_time(east).contains('T'));
        assert!(aws_timestamp() > 0);
    }

    #[test]
    fn creates_graphql_id_values() {
        let id = make_id().expect("ID should be generated");
        let next_id = make_id().expect("ID should be generated");

        assert_eq!(id.len(), 36);
        assert_eq!(id.matches('-').count(), 4);
        assert_eq!(id.as_bytes()[14], b'4');
        assert!(matches!(id.as_bytes()[19], b'8' | b'9' | b'a' | b'b'));
        assert_ne!(id, next_id);
    }
}
