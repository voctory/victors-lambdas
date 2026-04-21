//! Idempotency records.

use std::time::SystemTime;

use crate::{IdempotencyKey, IdempotencyStatus};

/// Stored state for one idempotent operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IdempotencyRecord {
    key: IdempotencyKey,
    status: IdempotencyStatus,
    expires_at: SystemTime,
    payload_hash: Option<String>,
    response_data: Option<Vec<u8>>,
}

impl IdempotencyRecord {
    /// Creates an in-progress idempotency record that expires at a fixed time.
    #[must_use]
    pub fn in_progress_until(key: impl Into<IdempotencyKey>, expires_at: SystemTime) -> Self {
        Self {
            key: key.into(),
            status: IdempotencyStatus::InProgress,
            expires_at,
            payload_hash: None,
            response_data: None,
        }
    }

    /// Creates a completed idempotency record that expires at a fixed time.
    #[must_use]
    pub fn completed_until(key: impl Into<IdempotencyKey>, expires_at: SystemTime) -> Self {
        Self {
            key: key.into(),
            status: IdempotencyStatus::Completed,
            expires_at,
            payload_hash: None,
            response_data: None,
        }
    }

    /// Returns the idempotency key.
    #[must_use]
    pub const fn key(&self) -> &IdempotencyKey {
        &self.key
    }

    /// Returns the persisted record status without applying expiry.
    #[must_use]
    pub const fn status(&self) -> IdempotencyStatus {
        self.status
    }

    /// Returns the effective status at `now`.
    #[must_use]
    pub fn status_at(&self, now: SystemTime) -> IdempotencyStatus {
        if self.is_expired_at(now) {
            IdempotencyStatus::Expired
        } else {
            self.status
        }
    }

    /// Returns when this record expires.
    #[must_use]
    pub const fn expires_at(&self) -> SystemTime {
        self.expires_at
    }

    /// Returns whether the record is expired at `now`.
    #[must_use]
    pub fn is_expired_at(&self, now: SystemTime) -> bool {
        now.duration_since(self.expires_at).is_ok()
    }

    /// Returns the optional payload hash.
    #[must_use]
    pub fn payload_hash(&self) -> Option<&str> {
        self.payload_hash.as_deref()
    }

    /// Returns the optional cached response data.
    #[must_use]
    pub fn response_data(&self) -> Option<&[u8]> {
        self.response_data.as_deref()
    }

    /// Returns a copy of this record with a payload hash.
    #[must_use]
    pub fn with_payload_hash(mut self, payload_hash: impl Into<String>) -> Self {
        self.payload_hash = Some(payload_hash.into());
        self
    }

    /// Returns a copy of this record with cached response data.
    #[must_use]
    pub fn with_response_data(mut self, response_data: impl Into<Vec<u8>>) -> Self {
        self.response_data = Some(response_data.into());
        self
    }

    /// Marks this record as completed and updates its expiry time.
    pub fn mark_completed(&mut self, expires_at: SystemTime) {
        self.status = IdempotencyStatus::Completed;
        self.expires_at = expires_at;
    }

    /// Consumes the record and returns its idempotency key.
    #[must_use]
    pub fn into_key(self) -> IdempotencyKey {
        self.key
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, UNIX_EPOCH};

    use super::IdempotencyRecord;
    use crate::IdempotencyStatus;

    #[test]
    fn in_progress_record_expires_at_boundary() {
        let expires_at = UNIX_EPOCH + Duration::from_secs(10);
        let record = IdempotencyRecord::in_progress_until("request-1", expires_at);

        assert_eq!(record.status(), IdempotencyStatus::InProgress);
        assert_eq!(
            record.status_at(expires_at - Duration::from_secs(1)),
            IdempotencyStatus::InProgress
        );
        assert_eq!(record.status_at(expires_at), IdempotencyStatus::Expired);
    }

    #[test]
    fn completed_record_can_cache_hash_and_response() {
        let record = IdempotencyRecord::completed_until("request-1", UNIX_EPOCH)
            .with_payload_hash("hash")
            .with_response_data(b"response".to_vec());

        assert_eq!(record.key().value(), "request-1");
        assert_eq!(record.payload_hash(), Some("hash"));
        assert_eq!(record.response_data(), Some(&b"response"[..]));
    }

    #[test]
    fn mark_completed_updates_status_and_expiry() {
        let mut record = IdempotencyRecord::in_progress_until("request-1", UNIX_EPOCH);
        let completed_expiry = UNIX_EPOCH + Duration::from_secs(60);

        record.mark_completed(completed_expiry);

        assert_eq!(record.status(), IdempotencyStatus::Completed);
        assert_eq!(record.expires_at(), completed_expiry);
    }
}
