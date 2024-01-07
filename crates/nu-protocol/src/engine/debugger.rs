use std::fmt::Debug;
use std::time::SystemTime;

use crate::ast::Call;
use crate::engine::{Command, EngineState, Stack};
use crate::{PipelineData, ShellError};

/// Trait for static dispatching of eval_xxx() and debugger callback calls
pub trait DebugContext: Clone + Copy {
    #[allow(unused_variables)]
    fn on_block_enter(&self, debugger: &mut dyn Debugger) {}
}

/// Marker struct signalizing that evaluation should use a Debugger
#[derive(Clone, Copy)]
pub struct WithDebug;

impl DebugContext for WithDebug {
    fn on_block_enter(&self, debugger: &mut dyn Debugger) {
        debugger.on_block_enter()
    }
}

/// Marker struct signalizing that evaluation should NOT use a Debugger
#[derive(Clone, Copy)]
pub struct WithoutDebug;

impl DebugContext for WithoutDebug {}

/// Debugger trait that every debugger needs to implement.
///
/// By default, its callbacks are empty.
pub trait Debugger {
    fn run_cmd(
        &mut self,
        decl: &Box<dyn Command>,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError>;

    fn on_block_enter(&mut self) {}
}

/// Basic debugger showcasing the functionality
pub struct BasicDebugger {
    pub timestamps: Vec<SystemTime>
}

impl Debugger for BasicDebugger {
    fn run_cmd(
        &mut self,
        decl: &Box<dyn Command>,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        decl.run_debug(engine_state, stack, call, input, self)
    }

    fn on_block_enter(&mut self) {
        self.timestamps.push(SystemTime::now());
        println!("Entered block with debugger!");
    }
}

/// Noop debugger doing nothing, should not interfere with normal flow in any way.
pub struct NoopDebugger;

impl Debugger for NoopDebugger {
    fn run_cmd(
        &mut self,
        decl: &Box<dyn Command>,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        decl.run(engine_state, stack, call, input)
    }
}
