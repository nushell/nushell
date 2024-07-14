use log::info;
use nu_engine::{convert_env_values, eval_block};
use nu_parser::parse;
use nu_protocol::{
    debugger::WithoutDebug,
    engine::{EngineState, Stack, StateWorkingSet},
    report_error, PipelineData, ShellError, Spanned, Value,
};
use std::sync::Arc;

#[derive(Default)]
pub struct EvaluateCommandsOpts {
    pub table_mode: Option<Value>,
    pub error_style: Option<Value>,
    pub no_newline: bool,
}

/// Run a command (or commands) given to us by the user
pub fn evaluate_commands(
    commands: &Spanned<String>,
    engine_state: &mut EngineState,
    stack: &mut Stack,
    input: PipelineData,
    opts: EvaluateCommandsOpts,
) -> Result<(), ShellError> {
    let EvaluateCommandsOpts {
        table_mode,
        error_style,
        no_newline,
    } = opts;

    // Handle the configured error style early
    if let Some(e_style) = error_style {
        match e_style.coerce_str()?.parse() {
            Ok(e_style) => {
                Arc::make_mut(&mut engine_state.config).error_style = e_style;
            }
            Err(err) => {
                return Err(ShellError::GenericError {
                    error: "Invalid value for `--error-style`".into(),
                    msg: err.into(),
                    span: Some(e_style.span()),
                    help: None,
                    inner: vec![],
                });
            }
        }
    }

    // Translate environment variables from Strings to Values
    convert_env_values(engine_state, stack)?;

    // Parse the source code
    let (block, delta) = {
        if let Some(ref t_mode) = table_mode {
            Arc::make_mut(&mut engine_state.config).table_mode =
                t_mode.coerce_str()?.parse().unwrap_or_default();
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

        if let Some(err) = working_set.compile_errors.first() {
            report_error(&working_set, err);
            // Not a fatal error, for now
        }

        (output, working_set.render())
    };

    // Update permanent state
    engine_state.merge_delta(delta)?;

    // Run the block
    let pipeline = eval_block::<WithoutDebug>(engine_state, stack, &block, input)?;

    if let PipelineData::Value(Value::Error { error, .. }, ..) = pipeline {
        return Err(*error);
    }

    if let Some(t_mode) = table_mode {
        Arc::make_mut(&mut engine_state.config).table_mode =
            t_mode.coerce_str()?.parse().unwrap_or_default();
    }

    if let Some(status) = pipeline.print(engine_state, stack, no_newline, false)? {
        if status.code() != 0 {
            std::process::exit(status.code())
        }
    }

    info!("evaluate {}:{}:{}", file!(), line!(), column!());

    Ok(())
}
