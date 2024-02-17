use crate::ast::PipelineElement;
use crate::engine::EngineState;
use crate::{PipelineData, ShellError, Span, Value};
use std::fmt::Debug;
use std::ops::DerefMut;

/// Trait for static dispatching of eval_xxx() and debugger callback calls
pub trait DebugContext: Clone + Copy + Debug {
    #[allow(unused_variables)]
    fn enter_block(engine_state: &EngineState) {}

    #[allow(unused_variables)]
    fn leave_block(engine_state: &EngineState) {}

    #[allow(unused_variables)]
    fn enter_element(engine_state: &EngineState) {}

    #[allow(unused_variables)]
    fn leave_element(
        engine_state: &EngineState,
        input: &Result<(PipelineData, bool), ShellError>,
        element: &PipelineElement,
    ) {
    }
}

/// Marker struct signalizing that evaluation should use a Debugger
#[derive(Clone, Copy, Debug)]
pub struct WithDebug;

impl DebugContext for WithDebug {
    fn enter_block(engine_state: &EngineState) {
        if let Ok(mut debugger) = engine_state.debugger.lock() {
            debugger.deref_mut().enter_block();
        }
    }

    fn leave_block(engine_state: &EngineState) {
        if let Ok(mut debugger) = engine_state.debugger.lock() {
            debugger.deref_mut().leave_block();
        }
    }

    fn enter_element(engine_state: &EngineState) {
        if let Ok(mut debugger) = engine_state.debugger.lock() {
            debugger.deref_mut().enter_element();
        }
    }

    fn leave_element(
        engine_state: &EngineState,
        input: &Result<(PipelineData, bool), ShellError>,
        element: &PipelineElement,
    ) {
        if let Ok(mut debugger) = engine_state.debugger.lock() {
            debugger
                .deref_mut()
                .leave_element(engine_state, input, element);
        }
    }
}

/// Marker struct signalizing that evaluation should NOT use a Debugger
#[derive(Clone, Copy, Debug)]
pub struct WithoutDebug;

impl DebugContext for WithoutDebug {}

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

    fn report(&self, profiler_span: Span) -> Result<Value, ShellError> {
        Ok(Value::nothing(profiler_span))
    }
}

/// Noop debugger doing nothing, should not interfere with normal flow in any way.
#[derive(Debug)]
pub struct NoopDebugger;

impl Debugger for NoopDebugger {}
