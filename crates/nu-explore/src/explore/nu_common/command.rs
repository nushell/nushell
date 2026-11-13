use nu_engine::eval_block;
use nu_parser::parse;
use nu_protocol::{
    OutDest, PipelineData, ShellError, Value,
    debugger::WithoutDebug,
    engine::{EngineState, Redirection, Stack, StateWorkingSet},
    shell_error::generic::GenericError,
};

pub fn run_command_with_value(
    command: &str,
    input: &Value,
    engine_state: &EngineState,
    stack: &mut Stack,
) -> Result<PipelineData, ShellError> {
    if is_ignored_command(command) {
        return Err(ShellError::Generic(GenericError::new_internal(
            "Command ignored",
            "the command is ignored",
        )));
    }

    let pipeline = PipelineData::value(input.clone(), None);
    let pipeline = run_nu_command(engine_state, stack, command, pipeline)?;
    if let PipelineData::Value(Value::Error { error, .. }, ..) = pipeline {
        Err(ShellError::Generic(
            GenericError::new_internal("Error from pipeline", error.to_string())
                .with_inner([*error]),
        ))
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
    let ignore_list: &[&str] = &["clear", "explore", "exit", "nu"];

    command
        .split_whitespace()
        .any(|token| ignore_list.contains(&token))
}

fn eval_source2(
    engine_state: &mut EngineState,
    stack: &mut Stack,
    source: &[u8],
    fname: &str,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let (block, delta) = {
        let mut working_set = StateWorkingSet::new(engine_state);
        let output = parse(
            &mut working_set,
            Some(fname), // format!("repl_entry #{}", entry_num)
            source,
            false,
        );

        // The pager prints an error's headline and nothing else, so that's where the reason goes.
        if let Some(err) = working_set.parse_errors.first() {
            return Err(ShellError::Generic(GenericError::new_internal(
                format!("Parse error: {err}"),
                "",
            )));
        }

        if let Some(err) = working_set.compile_errors.first() {
            return Err(ShellError::Generic(GenericError::new_internal(
                format!("Compile error: {err}"),
                "",
            )));
        }

        (output, working_set.render())
    };

    // We need to merge different info other wise things like PIPEs etc will not work.
    if let Err(err) = engine_state.merge_delta(delta) {
        return Err(ShellError::Generic(
            GenericError::new_internal("Merge error", err.to_string()).with_inner([err]),
        ));
    }

    // Every pipeline runs, and only the last one's output comes back. These redirections apply only
    // to the last one. The pager's stack discards the output of the others.
    let stack = &mut stack.push_redirection(
        Some(Redirection::Pipe(OutDest::PipeSeparate)),
        Some(Redirection::Pipe(OutDest::PipeSeparate)),
    );
    eval_block::<WithoutDebug>(engine_state, stack, &block, input).map(|p| p.body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nu_protocol::Span;

    #[test]
    fn run_nu_command_runs_every_pipeline() {
        let output = run_nu_command(
            &EngineState::new(),
            &mut Stack::new(),
            "$env.X = 2; $env.X * 3",
            PipelineData::empty(),
        )
        .and_then(|data| data.into_value(Span::test_data()));

        assert_eq!(output.ok(), Some(Value::test_int(6)));
    }

    #[test]
    fn run_nu_command_names_the_compile_error() {
        // An empty engine has no `run-external`, so this parses but doesn't compile.
        let err = run_nu_command(
            &EngineState::new(),
            &mut Stack::new(),
            "^echo hi",
            PipelineData::empty(),
        )
        .err()
        .map(|err| err.to_string());

        assert_eq!(
            err.as_deref(),
            Some("Compile error: External calls are not supported.")
        );
    }

    #[test]
    fn is_ignored_command_cases() {
        let cases = [
            ("Simple", "clear", true),
            ("With flag", "clear -x", true),
            ("Prepended", "sudo clear", true),
            ("Appended", "exit now", true),
            ("Valid", "ls -la", false),
            ("Starts with ignored", "clearly ok", false),
            (
                "Starts with ignored existing command",
                "nu_highlight",
                false,
            ),
            ("Empty string", "", false),
            ("Spaces only", "   ", false),
        ];

        for (name, input, expected) in cases {
            assert_eq!(
                is_ignored_command(input),
                expected,
                "Case failed for {name}: {input:?}"
            );
        }
    }
}
