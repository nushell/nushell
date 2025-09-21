//! Traits related to debugging
//!
//! The purpose of DebugContext is achieving static dispatch on `eval_xxx()` calls.
//! The main Debugger trait is intended to be used as a trait object.
//!
//! The debugging information is stored in `EngineState` as the `debugger` field storing a `Debugger`
//! trait object behind `Arc` and `Mutex`. To evaluate something (e.g., a block), first create a
//! `Debugger` trait object (such as the `Profiler`). Then, add it to engine state via
//! `engine_state.activate_debugger()`. This sets the internal state of EngineState to the debugging
//! mode and calls `Debugger::activate()`. Now, you can call `eval_xxx::<WithDebug>()`. When you're
//! done, call `engine_state.deactivate_debugger()` which calls `Debugger::deactivate()`, sets the
//! EngineState into non-debugging mode, and returns the original mutated `Debugger` trait object.
//! (`NoopDebugger` is placed in its place inside `EngineState`.) After deactivating, you can call
//! `Debugger::report()` to get some output from the debugger, if necessary.

use crate::{
    PipelineData, PipelineExecutionData, ShellError, Span, Value,
    ast::{Block, PipelineElement},
    engine::EngineState,
    ir::IrBlock,
};
use std::{fmt::Debug, ops::DerefMut};

/// Trait used for static dispatch of `eval_xxx()` evaluator calls
///
/// DebugContext implements the same interface as Debugger (except activate() and deactivate(). It
/// is intended to be implemented only by two structs
/// * WithDebug which calls down to the Debugger methods
/// * WithoutDebug with default implementation, i.e., empty calls to be optimized away
pub trait DebugContext: Clone + Copy + Debug {
    /// Called when the evaluator enters a block
    #[allow(unused_variables)]
    fn enter_block(engine_state: &EngineState, block: &Block) {}

    /// Called when the evaluator leaves a block
    #[allow(unused_variables)]
    fn leave_block(engine_state: &EngineState, block: &Block) {}

    /// Called when the AST evaluator enters a pipeline element
    #[allow(unused_variables)]
    fn enter_element(engine_state: &EngineState, element: &PipelineElement) {}

    /// Called when the AST evaluator leaves a pipeline element
    #[allow(unused_variables)]
    fn leave_element(
        engine_state: &EngineState,
        element: &PipelineElement,
        result: &Result<PipelineData, ShellError>,
    ) {
    }

    /// Called before the IR evaluator runs an instruction
    #[allow(unused_variables)]
    fn enter_instruction(
        engine_state: &EngineState,
        ir_block: &IrBlock,
        instruction_index: usize,
        registers: &[PipelineExecutionData],
    ) {
    }

    /// Called after the IR evaluator runs an instruction
    #[allow(unused_variables)]
    fn leave_instruction(
        engine_state: &EngineState,
        ir_block: &IrBlock,
        instruction_index: usize,
        registers: &[PipelineExecutionData],
        error: Option<&ShellError>,
    ) {
    }
}

/// Marker struct signalizing that evaluation should use a Debugger
///
/// Trait methods call to Debugger trait object inside the supplied EngineState.
#[derive(Clone, Copy, Debug)]
pub struct WithDebug;

impl DebugContext for WithDebug {
    fn enter_block(engine_state: &EngineState, block: &Block) {
        if let Ok(mut debugger) = engine_state.debugger.lock() {
            debugger.deref_mut().enter_block(engine_state, block);
        }
    }

    fn leave_block(engine_state: &EngineState, block: &Block) {
        if let Ok(mut debugger) = engine_state.debugger.lock() {
            debugger.deref_mut().leave_block(engine_state, block);
        }
    }

    fn enter_element(engine_state: &EngineState, element: &PipelineElement) {
        if let Ok(mut debugger) = engine_state.debugger.lock() {
            debugger.deref_mut().enter_element(engine_state, element);
        }
    }

    fn leave_element(
        engine_state: &EngineState,
        element: &PipelineElement,
        result: &Result<PipelineData, ShellError>,
    ) {
        if let Ok(mut debugger) = engine_state.debugger.lock() {
            debugger
                .deref_mut()
                .leave_element(engine_state, element, result);
        }
    }

    fn enter_instruction(
        engine_state: &EngineState,
        ir_block: &IrBlock,
        instruction_index: usize,
        registers: &[PipelineExecutionData],
    ) {
        if let Ok(mut debugger) = engine_state.debugger.lock() {
            debugger.deref_mut().enter_instruction(
                engine_state,
                ir_block,
                instruction_index,
                registers,
            )
        }
    }

    fn leave_instruction(
        engine_state: &EngineState,
        ir_block: &IrBlock,
        instruction_index: usize,
        registers: &[PipelineExecutionData],
        error: Option<&ShellError>,
    ) {
        if let Ok(mut debugger) = engine_state.debugger.lock() {
            debugger.deref_mut().leave_instruction(
                engine_state,
                ir_block,
                instruction_index,
                registers,
                error,
            )
        }
    }
}

/// Marker struct signalizing that evaluation should NOT use a Debugger
///
/// Trait methods are empty calls to be optimized away.
#[derive(Clone, Copy, Debug)]
pub struct WithoutDebug;

impl DebugContext for WithoutDebug {}

/// Debugger trait that every debugger needs to implement.
///
/// By default, its methods are empty. Not every Debugger needs to implement all of them.
pub trait Debugger: Send + Debug {
    /// Called by EngineState::activate_debugger().
    ///
    /// Intended for initializing the debugger.
    fn activate(&mut self) {}

    /// Called by EngineState::deactivate_debugger().
    ///
    /// Intended for wrapping up the debugger after a debugging session before returning back to
    /// normal evaluation without debugging.
    fn deactivate(&mut self) {}

    /// Called when the evaluator enters a block
    #[allow(unused_variables)]
    fn enter_block(&mut self, engine_state: &EngineState, block: &Block) {}

    /// Called when the evaluator leaves a block
    #[allow(unused_variables)]
    fn leave_block(&mut self, engine_state: &EngineState, block: &Block) {}

    /// Called when the AST evaluator enters a pipeline element
    #[allow(unused_variables)]
    fn enter_element(&mut self, engine_state: &EngineState, pipeline_element: &PipelineElement) {}

    /// Called when the AST evaluator leaves a pipeline element
    #[allow(unused_variables)]
    fn leave_element(
        &mut self,
        engine_state: &EngineState,
        element: &PipelineElement,
        result: &Result<PipelineData, ShellError>,
    ) {
    }

    /// Called before the IR evaluator runs an instruction
    #[allow(unused_variables)]
    fn enter_instruction(
        &mut self,
        engine_state: &EngineState,
        ir_block: &IrBlock,
        instruction_index: usize,
        registers: &[PipelineExecutionData],
    ) {
    }

    /// Called after the IR evaluator runs an instruction
    #[allow(unused_variables)]
    fn leave_instruction(
        &mut self,
        engine_state: &EngineState,
        ir_block: &IrBlock,
        instruction_index: usize,
        registers: &[PipelineExecutionData],
        error: Option<&ShellError>,
    ) {
    }

    /// Create a final report as a Value
    ///
    /// Intended to be called after deactivate()
    #[allow(unused_variables)]
    fn report(&self, engine_state: &EngineState, debugger_span: Span) -> Result<Value, ShellError> {
        Ok(Value::nothing(debugger_span))
    }
}

/// A debugger that does nothing
///
/// Used as a placeholder debugger when not debugging.
#[derive(Debug)]
pub struct NoopDebugger;

impl Debugger for NoopDebugger {}
