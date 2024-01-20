use crate::ast::PipelineElement;
use std::ops::DerefMut;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

/// Trait for static dispatching of eval_xxx() and debugger callback calls
pub trait DebugContext: Clone + Copy {
    fn should_debug(&self) -> bool;

    #[allow(unused_variables)]
    fn enter_block(&self, debugger: &Option<Arc<Mutex<dyn Debugger>>>) {}

    #[allow(unused_variables)]
    fn leave_block(&self, debugger: &Option<Arc<Mutex<dyn Debugger>>>) {}

    #[allow(unused_variables)]
    fn enter_element(
        &self,
        debugger: &Option<Arc<Mutex<dyn Debugger>>>,
        element: &PipelineElement,
    ) {
    }

    #[allow(unused_variables)]
    fn leave_element(
        &self,
        debugger: &Option<Arc<Mutex<dyn Debugger>>>,
        element: &PipelineElement,
    ) {
    }
}

/// Marker struct signalizing that evaluation should use a Debugger
#[derive(Clone, Copy)]
pub struct WithDebug;

// TODO: Remove unwraps
impl DebugContext for WithDebug {
    fn should_debug(&self) -> bool {
        true
    }
    fn enter_block(&self, debugger: &Option<Arc<Mutex<dyn Debugger>>>) {
        debugger
            .as_ref()
            .unwrap()
            .lock()
            .unwrap()
            .deref_mut()
            .enter_block();
    }

    fn leave_block(&self, debugger: &Option<Arc<Mutex<dyn Debugger>>>) {
        debugger
            .as_ref()
            .unwrap()
            .lock()
            .unwrap()
            .deref_mut()
            .leave_block();
    }

    fn enter_element(
        &self,
        debugger: &Option<Arc<Mutex<dyn Debugger>>>,
        element: &PipelineElement,
    ) {
        debugger
            .as_ref()
            .unwrap()
            .lock()
            .unwrap()
            .deref_mut()
            .enter_element(element);
    }

    fn leave_element(
        &self,
        debugger: &Option<Arc<Mutex<dyn Debugger>>>,
        element: &PipelineElement,
    ) {
        debugger
            .as_ref()
            .unwrap()
            .lock()
            .unwrap()
            .deref_mut()
            .leave_element(element);
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
    fn enter_block(&mut self) {}
    fn leave_block(&mut self) {}

    fn enter_element(&mut self, element: &PipelineElement) {}
    fn leave_element(&mut self, element: &PipelineElement) {}
}

/// Noop debugger doing nothing, should not interfere with normal flow in any way.
pub struct NoopDebugger;

impl Debugger for NoopDebugger {}
