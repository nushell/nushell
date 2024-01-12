use std::ops::DerefMut;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

/// Trait for static dispatching of eval_xxx() and debugger callback calls
pub trait DebugContext: Clone + Copy {
    #[allow(unused_variables)]
    fn on_block_enter(&self, debugger: &Option<Arc<Mutex<dyn Debugger>>>) {}

    #[allow(unused_variables)]
    fn on_block_leave(&self, debugger: &Option<Arc<Mutex<dyn Debugger>>>) {}
}

/// Marker struct signalizing that evaluation should use a Debugger
#[derive(Clone, Copy)]
pub struct WithDebug;

impl DebugContext for WithDebug {
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

impl DebugContext for WithoutDebug {}

/// Debugger trait that every debugger needs to implement.
///
/// By default, its callbacks are empty.
pub trait Debugger: Send {
    fn on_block_enter(&mut self) {}
    fn on_block_leave(&mut self) {}
}

/// Basic debugger showcasing the functionality
#[derive(Default)]
pub struct BasicDebugger {
    // pub data: BasicData
    pub instants: Vec<Instant>,
    pub durations_ms: Vec<u128>,
}

impl BasicDebugger {
    pub fn report(&self) {
        println!("Report ({} durations):", self.durations_ms.len());
        println!("=======");
        for duration in &self.durations_ms {
            println!("Duration: {:?} ms", duration);
        }
    }
}

impl Debugger for BasicDebugger {
    fn on_block_enter(&mut self) {
        self.instants.push(Instant::now());
        println!(
            "Entered block with debugger! {} timestamps, {} durations",
            self.instants.len(),
            self.durations_ms.len()
        );
    }

    fn on_block_leave(&mut self) {
        let start = self.instants.pop().unwrap();
        self.durations_ms.push(start.elapsed().as_millis());
        println!(
            "Left block with debugger! {} timestamps, {} durations",
            self.instants.len(),
            self.durations_ms.len()
        );
    }
}

/// Noop debugger doing nothing, should not interfere with normal flow in any way.
pub struct NoopDebugger;

impl Debugger for NoopDebugger {}
