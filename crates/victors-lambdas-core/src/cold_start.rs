//! Cold start tracking primitives.

use std::sync::atomic::{AtomicBool, Ordering};

static GLOBAL_COLD_START: ColdStart = ColdStart::new();

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

    /// Returns whether this tracker has already seen an invocation.
    #[must_use]
    pub fn has_seen_invocation(&self) -> bool {
        self.seen.load(Ordering::Acquire)
    }

    #[cfg(test)]
    fn reset_for_test(&self) {
        self.seen.store(false, Ordering::Release);
    }
}

impl Default for ColdStart {
    fn default() -> Self {
        Self::new()
    }
}

/// Returns the process-global cold-start tracker.
#[must_use]
pub fn global_tracker() -> &'static ColdStart {
    &GLOBAL_COLD_START
}

/// Marks an invocation on the process-global tracker.
///
/// Returns `true` only for the first call in the current process, which maps to the Lambda cold
/// start signal for a single execution environment.
pub fn is_cold_start() -> bool {
    global_tracker().mark_invocation()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static GLOBAL_TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn tracker_returns_true_only_for_first_invocation() {
        let tracker = ColdStart::new();

        assert!(!tracker.has_seen_invocation());
        assert!(tracker.mark_invocation());
        assert!(tracker.has_seen_invocation());
        assert!(!tracker.mark_invocation());
        assert!(tracker.has_seen_invocation());
    }

    #[test]
    fn global_helper_uses_shared_tracker() {
        let _guard = GLOBAL_TEST_LOCK
            .lock()
            .expect("global cold start test lock should not be poisoned");

        global_tracker().reset_for_test();

        let first = is_cold_start();
        let second = is_cold_start();

        assert!(first);
        assert!(!second);
        assert!(global_tracker().has_seen_invocation());

        global_tracker().reset_for_test();
    }
}
