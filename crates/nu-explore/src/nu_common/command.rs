use nu_engine::eval_block;
use nu_parser::parse;
use nu_protocol::{
    engine::{EngineState, Stack, StateWorkingSet},
    PipelineData, ShellError, Value,
};

pub fn run_command_with_value(
    command: &str,
    input: &Value,
    engine_state: &EngineState,
    stack: &mut Stack,
) -> Result<PipelineData, ShellError> {
    if is_ignored_command(command) {
        return Err(ShellError::IOError(String::from("the command is ignored")));
    }

    let pipeline = PipelineData::Value(input.clone(), None);
    let pipeline = run_nu_command(engine_state, stack, command, pipeline);
    match pipeline {
        Ok(PipelineData::Value(Value::Error { error }, ..)) => {
            Err(ShellError::IOError(error.to_string()))
        }
        Ok(pipeline) => Ok(pipeline),
        Err(err) => Err(err),
    }
}

pub fn run_nu_command(
    engine_state: &EngineState,
    stack: &mut Stack,
    cmd: &str,
    current: PipelineData,
) -> std::result::Result<PipelineData, ShellError> {
    let mut engine_state = engine_state.clone();
    eval_source2(&mut engine_state, stack, cmd.as_bytes(), "", current)
}

pub fn is_ignored_command(command: &str) -> bool {
    let ignore_list = ["clear", "explore", "exit"];

    for cmd in ignore_list {
        if command.starts_with(cmd) {
            return true;
        }
    }

    false
}

fn eval_source2(
    engine_state: &mut EngineState,
    stack: &mut Stack,
    source: &[u8],
    fname: &str,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let (mut block, delta) = {
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

    // We need to merge different info other wise things like PIPEs etc will not work.
    if let Err(err) = engine_state.merge_delta(delta) {
        return Err(ShellError::IOError(err.to_string()));
    }

    // eval_block outputs all expressions expept the last to STDOUT;
    // we don't wont that.
    //
    // So we LITERALLY ignore all expressions except the LAST.
    if block.len() > 1 {
        block.pipelines.drain(..block.pipelines.len() - 1);
    }

    eval_block(engine_state, stack, &block, input, true, true)
}
