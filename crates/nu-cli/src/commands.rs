use crate::util::report_error;
use log::info;
use miette::Result;
use nu_engine::{convert_env_values, eval_block};
use nu_parser::parse;
use nu_protocol::engine::Stack;
use nu_protocol::{
    engine::{EngineState, StateDelta, StateWorkingSet},
    PipelineData, Spanned, Value,
};
use std::path::Path;

pub fn evaluate_commands(
    commands: &Spanned<String>,
    init_cwd: &Path,
    engine_state: &mut EngineState,
    stack: &mut Stack,
    input: PipelineData,
    is_perf_true: bool,
    table_mode: Option<Value>,
) -> Result<()> {
    // Run a command (or commands) given to us by the user
    let (block, delta) = {
        if let Some(ref t_mode) = table_mode {
            let mut config = engine_state.get_config().clone();
            config.table_mode = t_mode.as_string()?;
            engine_state.set_config(&config);
        }

        let mut working_set = StateWorkingSet::new(engine_state);

        let (output, err) = parse(&mut working_set, None, commands.item.as_bytes(), false, &[]);
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

    let mut config = engine_state.get_config().clone();
    if let Some(t_mode) = table_mode {
        config.table_mode = t_mode.as_string()?;
    }

    // Merge the delta in case env vars changed in the config
    match nu_engine::env::current_dir(engine_state, stack) {
        Ok(cwd) => {
            if let Err(e) =
                engine_state.merge_delta(StateDelta::new(engine_state), Some(stack), cwd)
            {
                let working_set = StateWorkingSet::new(engine_state);
                report_error(&working_set, &e);
                std::process::exit(1);
            }
        }
        Err(e) => {
            let working_set = StateWorkingSet::new(engine_state);
            report_error(&working_set, &e);
            std::process::exit(1);
        }
    }

    // Translate environment variables from Strings to Values
    if let Some(e) = convert_env_values(engine_state, stack) {
        let working_set = StateWorkingSet::new(engine_state);
        report_error(&working_set, &e);
        std::process::exit(1);
    }

    match eval_block(engine_state, stack, &block, input, false, false) {
        Ok(pipeline_data) => {
            crate::eval_file::print_table_or_error(engine_state, stack, pipeline_data, &mut config)
        }
        Err(err) => {
            let working_set = StateWorkingSet::new(engine_state);

            report_error(&working_set, &err);
            std::process::exit(1);
        }
    }

    if is_perf_true {
        info!("evaluate {}:{}:{}", file!(), line!(), column!());
    }

    Ok(())
}
