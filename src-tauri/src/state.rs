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

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SetupTask {
    Frontend,
    Backend,
}

impl SetupTask {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "frontend" => Some(Self::Frontend),
            "backend" => Some(Self::Backend),
            _ => None,
        }
    }
}

/// The splash handshake: both halves report in, the window appears once.
#[derive(Default)]
pub struct SetupState {
    frontend_ready: bool,
    backend_ready: bool,
    revealed: bool,
}

impl SetupState {
    /// Records one half and answers whether the window should now be revealed.
    ///
    /// Recording and checking must stay one step. An earlier version let a
    /// caller set the flag itself: whichever half finished second never
    /// re-checked the condition, so a started app sat on the splash until the
    /// timeout fired.
    pub fn complete(&mut self, task: SetupTask) -> bool {
        match task {
            SetupTask::Frontend => self.frontend_ready = true,
            SetupTask::Backend => self.backend_ready = true,
        }
        self.frontend_ready && self.backend_ready
    }

    /// True exactly once, however often the handshake and the timeout race.
    pub fn claim_reveal(&mut self) -> bool {
        if self.revealed {
            return false;
        }
        self.revealed = true;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_window_waits_for_both_halves() {
        let mut state = SetupState::default();
        assert!(!state.complete(SetupTask::Frontend));
        assert!(state.complete(SetupTask::Backend));
    }

    #[test]
    fn only_the_first_caller_reveals() {
        let mut state = SetupState::default();
        assert!(state.claim_reveal());
        assert!(!state.claim_reveal());
    }

    #[test]
    fn the_guard_admits_one_scan() {
        let guard = ScanGuard::default();
        assert!(guard.try_claim());
        assert!(!guard.try_claim());
        guard.release();
        assert!(guard.try_claim());
    }
}
