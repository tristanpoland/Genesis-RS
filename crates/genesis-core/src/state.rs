//! Global application state management.

use std::sync::Arc;
use parking_lot::RwLock;

/// Global application state.
#[derive(Debug, Clone)]
pub struct State {
    /// Whether we're running in test mode
    pub under_test: bool,
    /// Whether we're in a callback
    pub in_callback: bool,
}

impl State {
    /// Create new state.
    pub fn new() -> Self {
        Self {
            under_test: false,
            in_callback: false,
        }
    }

    /// Get global state instance.
    pub fn global() -> Arc<RwLock<State>> {
        use once_cell::sync::Lazy;
        static INSTANCE: Lazy<Arc<RwLock<State>>> = Lazy::new(|| {
            Arc::new(RwLock::new(State::new()))
        });
        INSTANCE.clone()
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}
