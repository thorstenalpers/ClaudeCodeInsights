//! Everything registered with `.manage()`, in one place.

use std::sync::atomic::{AtomicBool, Ordering};

/// Keeps a second scan from starting while one is under way.
#[derive(Default)]
pub struct ScanGuard {
    running: AtomicBool,
}

impl ScanGuard {
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Claims the right to scan. False means someone else already holds it.
    pub fn try_claim(&self) -> bool {
        !self.running.swap(true, Ordering::SeqCst)
    }

    pub fn release(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_guard_admits_one_scan() {
        let guard = ScanGuard::default();
        assert!(guard.try_claim());
        assert!(!guard.try_claim());
        guard.release();
        assert!(guard.try_claim());
    }
}
