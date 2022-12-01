use nu_engine::eval_block;
use nu_parser::parse;
use nu_protocol::{
    engine::{EngineState, Stack, StateWorkingSet},
    PipelineData, ShellError,
};

pub fn run_nu_command(
    engine_state: &EngineState,
    stack: &mut Stack,
    cmd: &str,
    current: PipelineData,
) -> std::result::Result<PipelineData, ShellError> {
    let engine_state = engine_state.clone();
    eval_source2(&engine_state, stack, cmd.as_bytes(), "", current)
}

pub fn is_ignored_command(command: &str) -> bool {
    command.starts_with("clear")
}

fn eval_source2(
    engine_state: &EngineState,
    stack: &mut Stack,
    source: &[u8],
    fname: &str,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let (block, _) = {
        let mut working_set = StateWorkingSet::new(engine_state);
        let (output, err) = parse(
            &mut working_set,
            Some(fname), // format!("entry #{}", entry_num)
            source,
            false,
            &[],
        );
        if let Some(err) = err {
            return Err(ShellError::IOError(err.to_string()));
        }

        (output, working_set.render())
    };

    eval_block(engine_state, stack, &block, input, false, false)
}
