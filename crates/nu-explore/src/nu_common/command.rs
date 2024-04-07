use nu_engine::eval_block;
use nu_parser::parse;
use nu_protocol::{
    debugger::WithoutDebug,
    engine::{EngineState, Redirection, Stack, StateWorkingSet},
    OutDest, PipelineData, ShellError, Value,
};
use std::sync::Arc;

pub fn run_command_with_value(
    command: &str,
    input: &Value,
    engine_state: &EngineState,
    stack: &mut Stack,
) -> Result<PipelineData, ShellError> {
    if is_ignored_command(command) {
        return Err(ShellError::IOError {
            msg: String::from("the command is ignored"),
        });
    }

    let pipeline = PipelineData::Value(input.clone(), None);
    let pipeline = run_nu_command(engine_state, stack, command, pipeline)?;
    if let PipelineData::Value(Value::Error { error, .. }, ..) = pipeline {
        Err(ShellError::IOError {
            msg: error.to_string(),
        })
    } else {
        Ok(pipeline)
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
        let output = parse(
            &mut working_set,
            Some(fname), // format!("entry #{}", entry_num)
            source,
            false,
        );

        if let Some(err) = working_set.parse_errors.first() {
            return Err(ShellError::IOError {
                msg: err.to_string(),
            });
        }

        (output, working_set.render())
    };

    // We need to merge different info other wise things like PIPEs etc will not work.
    if let Err(err) = engine_state.merge_delta(delta) {
        return Err(ShellError::IOError {
            msg: err.to_string(),
        });
    }

    // eval_block outputs all expressions except the last to STDOUT;
    // we don't won't that.
    //
    // So we LITERALLY ignore all expressions except the LAST.
    if block.len() > 1 {
        let range = ..block.pipelines.len() - 1;
        // Note: `make_mut` will mutate `&mut block: &mut Arc<Block>`
        // for the outer fn scope `eval_block`
        Arc::make_mut(&mut block).pipelines.drain(range);
    }

    let stack = &mut stack.push_redirection(
        Some(Redirection::Pipe(OutDest::Capture)),
        Some(Redirection::Pipe(OutDest::Capture)),
    );
    eval_block::<WithoutDebug>(engine_state, stack, &block, input)
}
