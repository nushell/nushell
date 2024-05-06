use std::io::Write;

use log::info;
use miette::Result;
use nu_engine::{convert_env_values, eval_block};
use nu_parser::parse;
use nu_protocol::{
    debugger::WithoutDebug,
    engine::{EngineState, Stack, StateWorkingSet},
    report_error, Config, PipelineData, ShellError, Spanned, Value,
};

/// Run a command (or commands) given to us by the user
pub fn evaluate_commands(
    commands: &Spanned<String>,
    engine_state: &mut EngineState,
    stack: &mut Stack,
    input: PipelineData,
    table_mode: Option<Value>,
    no_newline: bool,
) -> Result<(), ShellError> {
    // Translate environment variables from Strings to Values
    convert_env_values(engine_state, stack)?;

    // Parse the source code
    let (block, delta) = {
        if let Some(ref t_mode) = table_mode {
            let mut config = engine_state.get_config().clone();
            config.table_mode = t_mode.coerce_str()?.parse().unwrap_or_default();
            engine_state.set_config(config);
        }

        let mut working_set = StateWorkingSet::new(engine_state);

        let output = parse(&mut working_set, None, commands.item.as_bytes(), false);
        if let Some(warning) = working_set.parse_warnings.first() {
            report_error(&working_set, warning);
        }

        if let Some(err) = working_set.parse_errors.first() {
            report_error(&working_set, err);
            std::process::exit(1);
        }

        (output, working_set.render())
    };

    // Update permanent state
    engine_state.merge_delta(delta)?;

    // Run the block
    let data = eval_block::<WithoutDebug>(engine_state, stack, &block, input)?;
    let mut config = engine_state.get_config().clone();
    if let Some(t_mode) = table_mode {
        config.table_mode = t_mode.coerce_str()?.parse().unwrap_or_default();
    }
    let exit_code = print_table_or_error(engine_state, stack, data, &mut config, no_newline);

    info!("evaluate {}:{}:{}", file!(), line!(), column!());

    match exit_code {
        None | Some(0) => Ok(()),
        Some(exit_code) => std::process::exit(exit_code as i32),
    }
}

pub(crate) fn print_table_or_error(
    engine_state: &mut EngineState,
    stack: &mut Stack,
    mut pipeline_data: PipelineData,
    config: &mut Config,
    no_newline: bool,
) -> Option<i64> {
    let exit_code = match &mut pipeline_data {
        PipelineData::ExternalStream { exit_code, .. } => exit_code.take(),
        _ => None,
    };

    // Change the engine_state config to use the passed in configuration
    engine_state.set_config(config.clone());

    if let PipelineData::Value(Value::Error { error, .. }, ..) = &pipeline_data {
        let working_set = StateWorkingSet::new(engine_state);
        report_error(&working_set, &**error);
        std::process::exit(1);
    }

    // We don't need to do anything special to print a table because print() handles it
    print_or_exit(pipeline_data, engine_state, stack, no_newline);

    // Make sure everything has finished
    if let Some(exit_code) = exit_code {
        let mut exit_code: Vec<_> = exit_code.into_iter().collect();
        exit_code
            .pop()
            .and_then(|last_exit_code| match last_exit_code {
                Value::Int { val: code, .. } => Some(code),
                _ => None,
            })
    } else {
        None
    }
}

fn print_or_exit(
    pipeline_data: PipelineData,
    engine_state: &EngineState,
    stack: &mut Stack,
    no_newline: bool,
) {
    let result = pipeline_data.print(engine_state, stack, no_newline, false);

    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();

    if let Err(error) = result {
        let working_set = StateWorkingSet::new(engine_state);
        report_error(&working_set, &error);
        let _ = std::io::stderr().flush();
        std::process::exit(1);
    }
}
