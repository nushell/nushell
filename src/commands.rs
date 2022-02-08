use miette::Result;
use nu_engine::{convert_env_values, eval_block};
use std::{io::Write, path::Path};

use nu_parser::{lex, lite_parse, parse_block, trim_quotes};
use nu_protocol::{
    ast::Call,
    engine::{EngineState, StateDelta, StateWorkingSet},
    Config, PipelineData, Span, Spanned, Value, CONFIG_VARIABLE_ID,
};

use crate::utils::{gather_parent_env_vars, report_error};

pub(crate) fn evaluate(
    commands: &Spanned<String>,
    init_cwd: &Path,
    engine_state: &mut EngineState,
    input: PipelineData,
) -> Result<()> {
    // First, set up env vars as strings only
    gather_parent_env_vars(engine_state);

    // Run a command (or commands) given to us by the user
    let (block, delta) = {
        let mut working_set = StateWorkingSet::new(engine_state);

        let (input, span_offset) =
            if commands.item.starts_with('\'') || commands.item.starts_with('"') {
                (
                    trim_quotes(commands.item.as_bytes()),
                    commands.span.start + 1,
                )
            } else {
                (commands.item.as_bytes(), commands.span.start)
            };

        let (output, err) = lex(input, span_offset, &[], &[], false);
        if let Some(err) = err {
            report_error(&working_set, &err);

            std::process::exit(1);
        }

        let (output, err) = lite_parse(&output);
        if let Some(err) = err {
            report_error(&working_set, &err);

            std::process::exit(1);
        }

        let (output, err) = parse_block(&mut working_set, &output, false);
        if let Some(err) = err {
            report_error(&working_set, &err);

            std::process::exit(1);
        }

        if let Some(err) = err {
            report_error(&working_set, &err);

            std::process::exit(1);
        }
        (output, working_set.render())
    };

    if let Err(err) = engine_state.merge_delta(delta, None, init_cwd) {
        let working_set = StateWorkingSet::new(engine_state);
        report_error(&working_set, &err);
    }

    let mut stack = nu_protocol::engine::Stack::new();

    // Set up our initial config to start from
    stack.vars.insert(
        CONFIG_VARIABLE_ID,
        Value::Record {
            cols: vec![],
            vals: vec![],
            span: Span { start: 0, end: 0 },
        },
    );

    let config = match stack.get_config() {
        Ok(config) => config,
        Err(e) => {
            let working_set = StateWorkingSet::new(engine_state);

            report_error(&working_set, &e);
            Config::default()
        }
    };

    // Merge the delta in case env vars changed in the config
    match nu_engine::env::current_dir(engine_state, &stack) {
        Ok(cwd) => {
            if let Err(e) = engine_state.merge_delta(StateDelta::new(), Some(&mut stack), cwd) {
                let working_set = StateWorkingSet::new(engine_state);
                report_error(&working_set, &e);
            }
        }
        Err(e) => {
            let working_set = StateWorkingSet::new(engine_state);
            report_error(&working_set, &e);
        }
    }

    // Translate environment variables from Strings to Values
    if let Some(e) = convert_env_values(engine_state, &stack, &config) {
        let working_set = StateWorkingSet::new(engine_state);
        report_error(&working_set, &e);
        std::process::exit(1);
    }

    match eval_block(engine_state, &mut stack, &block, input) {
        Ok(pipeline_data) => {
            match engine_state.find_decl("table".as_bytes()) {
                Some(decl_id) => {
                    let table = engine_state.get_decl(decl_id).run(
                        engine_state,
                        &mut stack,
                        &Call::new(Span::new(0, 0)),
                        pipeline_data,
                    )?;

                    for item in table {
                        let stdout = std::io::stdout();

                        if let Value::Error { error } = item {
                            let working_set = StateWorkingSet::new(engine_state);

                            report_error(&working_set, &error);

                            std::process::exit(1);
                        }

                        let mut out = item.into_string("\n", &config);
                        out.push('\n');

                        match stdout.lock().write_all(out.as_bytes()) {
                            Ok(_) => (),
                            Err(err) => eprintln!("{}", err),
                        };
                    }
                }
                None => {
                    for item in pipeline_data {
                        let stdout = std::io::stdout();

                        if let Value::Error { error } = item {
                            let working_set = StateWorkingSet::new(engine_state);

                            report_error(&working_set, &error);

                            std::process::exit(1);
                        }

                        let mut out = item.into_string("\n", &config);
                        out.push('\n');

                        match stdout.lock().write_all(out.as_bytes()) {
                            Ok(_) => (),
                            Err(err) => eprintln!("{}", err),
                        };
                    }
                }
            };
        }
        Err(err) => {
            let working_set = StateWorkingSet::new(engine_state);

            report_error(&working_set, &err);

            std::process::exit(1);
        }
    }

    Ok(())
}
