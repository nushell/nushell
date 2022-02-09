use crate::is_perf_true;
use crate::utils::{gather_parent_env_vars, report_error};
use log::info;
use miette::Result;
use nu_engine::{convert_env_values, eval_block};
use nu_parser::{lex, lite_parse, parse_block, trim_quotes};
use nu_protocol::{
    engine::{EngineState, StateDelta, StateWorkingSet},
    Config, PipelineData, Span, Spanned, Value, CONFIG_VARIABLE_ID,
};
use std::path::Path;

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
            crate::eval_file::print_table_or_error(engine_state, &mut stack, pipeline_data, &config)
        }
        Err(err) => {
            let working_set = StateWorkingSet::new(engine_state);

            report_error(&working_set, &err);

            std::process::exit(1);
        }
    }

    if is_perf_true() {
        info!("evaluate {}:{}:{}", file!(), line!(), column!());
    }

    Ok(())
}
