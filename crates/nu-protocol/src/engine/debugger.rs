use std::ops::DerefMut;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

/// Trait for static dispatching of eval_xxx() and debugger callback calls
pub trait DebugContext: Clone + Copy {
    #[allow(unused_variables)]
    fn on_block_enter(&self, debugger: Option<Arc<Mutex<dyn Debugger>>>) {}
}

/// Marker struct signalizing that evaluation should use a Debugger
#[derive(Clone, Copy)]
pub struct WithDebug;

impl DebugContext for WithDebug {
    fn on_block_enter(&self, debugger: Option<Arc<Mutex<dyn Debugger>>>) {
        debugger.unwrap().lock().unwrap().deref_mut().on_block_enter();
    }
}

/// Marker struct signalizing that evaluation should NOT use a Debugger
#[derive(Clone, Copy)]
pub struct WithoutDebug;

impl DebugContext for WithoutDebug {}

/// Debugger trait that every debugger needs to implement.
///
/// By default, its callbacks are empty.
pub trait Debugger: Send {
    fn on_block_enter(&mut self) {}
}

/// Basic debugger showcasing the functionality
#[derive(Default)]
pub struct BasicDebugger {
    // pub data: BasicData
    pub timestamps: Vec<SystemTime>
}

impl Debugger for BasicDebugger {
    fn on_block_enter(&mut self) {
        self.timestamps.push(SystemTime::now());
        println!("Entered block with debugger!");
    }
}

/// Noop debugger doing nothing, should not interfere with normal flow in any way.
pub struct NoopDebugger;

impl Debugger for NoopDebugger { }
