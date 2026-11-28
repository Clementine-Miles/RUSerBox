//! Window availability and Activity visibility change independently on Android.

#[derive(Default)]
pub(crate) struct WindowState {
    resumed: bool,
    pending_probe: bool,
}

impl WindowState {
    pub fn set_resumed(&mut self, resumed: bool) {
        self.resumed = resumed;
    }

    pub fn set_window(&mut self, available: bool) {
        self.pending_probe = available;
    }

    pub fn take_probe(&mut self) -> bool {
        self.resumed && std::mem::take(&mut self.pending_probe)
    }
}

#[cfg(test)]
mod tests {
    use super::WindowState;

    #[test]
    fn window_can_arrive_before_resume() {
        let mut lifecycle = WindowState::default();
        lifecycle.set_window(true);
        assert!(!lifecycle.take_probe());
        lifecycle.set_resumed(true);
        assert!(lifecycle.take_probe());
        assert!(!lifecycle.take_probe());
    }

    #[test]
    fn resume_can_arrive_before_window() {
        let mut lifecycle = WindowState::default();
        lifecycle.set_resumed(true);
        assert!(!lifecycle.take_probe());
        lifecycle.set_window(true);
        assert!(lifecycle.take_probe());
        lifecycle.set_resumed(false);
        lifecycle.set_resumed(true);
        assert!(!lifecycle.take_probe());
    }

    #[test]
    fn terminated_background_window_is_never_probed() {
        let mut lifecycle = WindowState::default();
        lifecycle.set_window(true);
        lifecycle.set_window(false);
        lifecycle.set_resumed(true);
        assert!(!lifecycle.take_probe());
        lifecycle.set_window(true);
        assert!(lifecycle.take_probe());
    }
}
