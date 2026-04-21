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

impl IdempotencyStatus {
    /// Returns whether this status represents completed handler work.
    #[must_use]
    pub const fn is_completed(self) -> bool {
        matches!(self, Self::Completed)
    }

    /// Returns whether this status represents in-flight handler work.
    #[must_use]
    pub const fn is_in_progress(self) -> bool {
        matches!(self, Self::InProgress)
    }

    /// Returns whether this status represents an expired record.
    #[must_use]
    pub const fn is_expired(self) -> bool {
        matches!(self, Self::Expired)
    }

    /// Returns whether this status is terminal for the current attempt.
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Expired)
    }
}

#[cfg(test)]
mod tests {
    use super::IdempotencyStatus;

    #[test]
    fn status_predicates_describe_state() {
        assert!(IdempotencyStatus::Completed.is_completed());
        assert!(IdempotencyStatus::InProgress.is_in_progress());
        assert!(IdempotencyStatus::Expired.is_expired());
        assert!(IdempotencyStatus::Completed.is_terminal());
        assert!(IdempotencyStatus::Expired.is_terminal());
        assert!(!IdempotencyStatus::InProgress.is_terminal());
    }
}
