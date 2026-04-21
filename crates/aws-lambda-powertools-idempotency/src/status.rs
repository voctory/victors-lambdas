//! Idempotency record status values.

/// State of an idempotency record.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IdempotencyStatus {
    /// Work is currently in progress.
    InProgress,
    /// Work completed successfully.
    Completed,
    /// Work expired and may be retried.
    Expired,
}
