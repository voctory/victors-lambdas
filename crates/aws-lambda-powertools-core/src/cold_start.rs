//! Cold start tracking primitives.

use std::sync::atomic::{AtomicBool, Ordering};

/// Tracks whether a Lambda execution environment has handled its first invocation.
#[derive(Debug)]
pub struct ColdStart {
    seen: AtomicBool,
}

impl ColdStart {
    /// Creates a new cold start tracker.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            seen: AtomicBool::new(false),
        }
    }

    /// Marks an invocation and returns `true` only for the first call.
    pub fn mark_invocation(&self) -> bool {
        !self.seen.swap(true, Ordering::AcqRel)
    }
}

impl Default for ColdStart {
    fn default() -> Self {
        Self::new()
    }
}
