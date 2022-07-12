use nu_cli::{eval_env_change_hook, eval_hook};
use nu_command::create_default_context;
use nu_engine::eval_block;
use nu_parser::parse;
use nu_protocol::engine::{EngineState, Stack, StateDelta, StateWorkingSet};
use nu_protocol::{CliError, PipelineData, Span, Value};
use nu_test_support::fs::in_directory;
use nu_test_support::Outcome;

fn outcome_err(
    engine_state: &EngineState,
    error: &(dyn miette::Diagnostic + Send + Sync + 'static),
) -> Outcome {
    let working_set = StateWorkingSet::new(engine_state);

    eprintln!("Error: {:?}", CliError(error, &working_set));

    Outcome {
        out: String::new(),
        err: format!("{:?}", error),
    }
}

fn outcome_ok(msg: String) -> Outcome {
    Outcome {
        out: msg,
        err: String::new(),
    }
}

pub fn nu_repl(cwd: &str, source_lines: &[&str]) -> Outcome {
    let cwd = in_directory(cwd);

    let mut engine_state = create_default_context(&cwd);
    let mut stack = Stack::new();

    stack.add_env_var(
        "PWD".to_string(),
        Value::String {
            val: cwd.to_string(),
            span: Span::test_data(),
        },
    );

    let delta = StateDelta::new(&engine_state);
    if let Err(err) = engine_state.merge_delta(delta, Some(&mut stack), cwd) {
        return outcome_err(&engine_state, &err);
    }

    let mut last_output = String::new();

    for (i, line) in source_lines.iter().enumerate() {
        // Check for pre_prompt hook
        let config = engine_state.get_config();
        if let Some(hook) = config.hooks.pre_prompt.clone() {
            if let Err(err) = eval_hook(&mut engine_state, &mut stack, vec![], &hook) {
                return outcome_err(&engine_state, &err);
            }
        }

        // Check for env change hook
        let config = engine_state.get_config();
        if let Err(err) = eval_env_change_hook(
            config.hooks.env_change.clone(),
            &mut engine_state,
            &mut stack,
        ) {
            return outcome_err(&engine_state, &err);
        }

        // Check for pre_execution hook
        let config = engine_state.get_config();
        if let Some(hook) = config.hooks.pre_execution.clone() {
            if let Err(err) = eval_hook(&mut engine_state, &mut stack, vec![], &hook) {
                return outcome_err(&engine_state, &err);
            }
        }

        // Eval the REPL line
        let (block, delta) = {
            let mut working_set = StateWorkingSet::new(&engine_state);
            let (block, err) = parse(
                &mut working_set,
                Some(&format!("line{}", i)),
                line.as_bytes(),
                false,
                &[],
            );

            if let Some(err) = err {
                return outcome_err(&engine_state, &err);
            }
            (block, working_set.render())
        };

        let cwd = match nu_engine::env::current_dir(&engine_state, &stack) {
            Ok(p) => p,
            Err(e) => {
                return outcome_err(&engine_state, &e);
            }
        };

        if let Err(err) = engine_state.merge_delta(delta, Some(&mut stack), &cwd) {
            return outcome_err(&engine_state, &err);
        }

        let input = PipelineData::new(Span::test_data());
        let config = engine_state.get_config();

        match eval_block(&engine_state, &mut stack, &block, input, false, false) {
            Ok(pipeline_data) => match pipeline_data.collect_string("", config) {
                Ok(s) => last_output = s,
                Err(err) => return outcome_err(&engine_state, &err),
            },
            Err(err) => return outcome_err(&engine_state, &err),
        }

        if let Some(cwd) = stack.get_env_var(&engine_state, "PWD") {
            let path = match cwd.as_string() {
                Ok(p) => p,
                Err(err) => return outcome_err(&engine_state, &err),
            };
            let _ = std::env::set_current_dir(path);
            engine_state.add_env_var("PWD".into(), cwd);
        }
    }

    outcome_ok(last_output)
}
