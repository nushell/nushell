use std::ops::DerefMut;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

/// Trait for static dispatching of eval_xxx() and debugger callback calls
pub trait DebugContext: Clone + Copy {
    fn should_debug(&self) -> bool;

    #[allow(unused_variables)]
    fn on_block_enter(&self, debugger: &Option<Arc<Mutex<dyn Debugger>>>) {}

    #[allow(unused_variables)]
    fn on_block_leave(&self, debugger: &Option<Arc<Mutex<dyn Debugger>>>) {}
}

/// Marker struct signalizing that evaluation should use a Debugger
#[derive(Clone, Copy)]
pub struct WithDebug;

impl DebugContext for WithDebug {
    fn should_debug(&self) -> bool {
        true
    }
    fn on_block_enter(&self, debugger: &Option<Arc<Mutex<dyn Debugger>>>) {
        debugger
            .as_ref()
            .unwrap()
            .lock()
            .unwrap()
            .deref_mut()
            .on_block_enter();
    }

    fn on_block_leave(&self, debugger: &Option<Arc<Mutex<dyn Debugger>>>) {
        debugger
            .as_ref()
            .unwrap()
            .lock()
            .unwrap()
            .deref_mut()
            .on_block_leave();
    }
}

/// Marker struct signalizing that evaluation should NOT use a Debugger
#[derive(Clone, Copy)]
pub struct WithoutDebug;

impl DebugContext for WithoutDebug {
    fn should_debug(&self) -> bool {
        false
    }
}

/// Debugger trait that every debugger needs to implement.
///
/// By default, its callbacks are empty.
pub trait Debugger: Send {
    fn on_block_enter(&mut self) {}
    fn on_block_leave(&mut self) {}
}

/// Noop debugger doing nothing, should not interfere with normal flow in any way.
pub struct NoopDebugger;

impl Debugger for NoopDebugger {}
