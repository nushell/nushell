use std::sync::Arc;

use nu_engine::eval_block;
use nu_parser::parse;
use nu_plugin::Plugin;
use nu_protocol::{PipelineData, engine::{EngineState, StateWorkingSet, Stack}, ShellError, debugger::WithoutDebug};

use crate::fake_register::create_engine_state;

/// An object through which plugins can be tested.
pub struct PluginTest {
    engine_state: EngineState,
    entry_num: usize,
}

impl PluginTest {
    /// Create a new test for the given `plugin` named `name`.
    pub fn new(name: &str, plugin: Arc<impl Plugin + Send + 'static>) -> Result<PluginTest, ShellError> {
        let engine_state = create_engine_state(name, plugin)?;
        Ok(PluginTest {
            engine_state,
            entry_num: 1,
        })
    }

    /// Evaluate some Nushell source code with the plugin commands in scope with the given input to
    /// the pipeline.
    pub fn eval_with(&mut self, nu_source: &str, input: PipelineData) -> Result<PipelineData, ShellError> {
        let mut working_set = StateWorkingSet::new(&self.engine_state);
        let fname = format!("entry #{}", self.entry_num);
        self.entry_num += 1;

        // Parse the source code and merge state
        let block = parse(&mut working_set, Some(&fname), nu_source.as_bytes(), false);
        self.engine_state.merge_delta(working_set.render())?;

        // Eval the block with the input
        let mut stack = Stack::new().capture();
        eval_block::<WithoutDebug>(&self.engine_state, &mut stack, &block, input)
    }

    /// Evaluate some Nushell source code with the plugin commands in scope.
    pub fn eval(&mut self, nu_source: &str) -> Result<PipelineData, ShellError> {
        self.eval_with(nu_source, PipelineData::Empty)
    }
}
