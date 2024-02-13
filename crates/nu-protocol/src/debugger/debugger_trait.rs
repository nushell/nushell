use crate::ast::PipelineElement;
use crate::engine::EngineState;
use crate::{PipelineData, ShellError};
use std::fmt::Debug;
use std::ops::DerefMut;
use std::sync::{Arc, Mutex};

/// Trait for static dispatching of eval_xxx() and debugger callback calls
pub trait DebugContext: Clone + Copy + Debug {
    fn should_debug(&self) -> bool;

    #[allow(unused_variables)]
    fn enter_block(&self, debugger: &Option<Arc<Mutex<dyn Debugger>>>) {}
    fn enter_block2(debugger: &Option<Arc<Mutex<dyn Debugger>>>) {}

    #[allow(unused_variables)]
    fn leave_block(&self, debugger: &Option<Arc<Mutex<dyn Debugger>>>) {}

    #[allow(unused_variables)]
    fn leave_element(
        &self,
        debugger: &Option<Arc<Mutex<dyn Debugger>>>,
        engine_state: &EngineState,
        input: &Result<(PipelineData, bool), ShellError>,
        element: &PipelineElement,
    ) {
    }

    #[allow(unused_variables)]
    fn enter_element(&self, debugger: &Option<Arc<Mutex<dyn Debugger>>>) {}
}

/// Marker struct signalizing that evaluation should use a Debugger
#[derive(Clone, Copy, Debug)]
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
    fn enter_block2(debugger: &Option<Arc<Mutex<dyn Debugger>>>) {
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

    fn leave_element(
        &self,
        debugger: &Option<Arc<Mutex<dyn Debugger>>>,
        engine_state: &EngineState,
        input: &Result<(PipelineData, bool), ShellError>,
        element: &PipelineElement,
    ) {
        debugger
            .as_ref()
            .unwrap()
            .lock()
            .unwrap()
            .deref_mut()
            .leave_element(engine_state, input, element);
    }

    fn enter_element(&self, debugger: &Option<Arc<Mutex<dyn Debugger>>>) {
        debugger
            .as_ref()
            .unwrap()
            .lock()
            .unwrap()
            .deref_mut()
            .enter_element();
    }
}

/// Marker struct signalizing that evaluation should NOT use a Debugger
#[derive(Clone, Copy, Debug)]
pub struct WithoutDebug;

impl DebugContext for WithoutDebug {
    fn should_debug(&self) -> bool {
        false
    }
}

/// Debugger trait that every debugger needs to implement.
///
/// By default, its callbacks are empty.
pub trait Debugger: Send + Debug {
    fn enter_block(&mut self) {}
    fn leave_block(&mut self) {}

    #[allow(unused_variables)]
    fn leave_element(
        &mut self,
        engine_state: &EngineState,
        input: &Result<(PipelineData, bool), ShellError>,
        element: &PipelineElement,
    ) {
    }
    fn enter_element(&mut self) {}
}

/// Noop debugger doing nothing, should not interfere with normal flow in any way.
#[derive(Debug)]
pub struct NoopDebugger;

impl Debugger for NoopDebugger {}
