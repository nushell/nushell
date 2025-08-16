use log::info;
use nu_engine::eval_block_track_exits;
use nu_parser::parse;
use nu_protocol::{
    PipelineData, ShellError, Spanned, Value,
    debugger::WithoutDebug,
    engine::{EngineState, Stack, StateWorkingSet},
    process::check_exit_status_future_ok,
    report_error::report_compile_error,
    report_parse_error, report_parse_warning,
};
use std::sync::Arc;

use crate::util::print_pipeline;

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

    // Parse the source code
    let (block, delta) = {
        if let Some(ref t_mode) = table_mode {
            Arc::make_mut(&mut engine_state.config).table.mode =
                t_mode.coerce_str()?.parse().unwrap_or_default();
        }

        let mut working_set = StateWorkingSet::new(engine_state);

        let output = parse(&mut working_set, None, commands.item.as_bytes(), false);
        if let Some(warning) = working_set.parse_warnings.first() {
            report_parse_warning(&working_set, warning);
        }

        if let Some(err) = working_set.parse_errors.first() {
            report_parse_error(&working_set, err);
            std::process::exit(1);
        }

        if let Some(err) = working_set.compile_errors.first() {
            report_compile_error(&working_set, err);
            std::process::exit(1);
        }

        (output, working_set.render())
    };

    // Update permanent state
    engine_state.merge_delta(delta)?;

    // Run the block
    let pipeline = eval_block_track_exits::<WithoutDebug>(engine_state, stack, &block, input)?;

    let (pipeline_data, exit_status) = (pipeline.body, pipeline.exit);
    if let PipelineData::Value(Value::Error { error, .. }, ..) = pipeline_data {
        return Err(*error);
    }

    if let Some(t_mode) = table_mode {
        Arc::make_mut(&mut engine_state.config).table.mode =
            t_mode.coerce_str()?.parse().unwrap_or_default();
    }

    print_pipeline(engine_state, stack, pipeline_data, no_newline)?;
    let pipefail = engine_state.get_config().pipefail;
    if !pipefail {
        return Ok(());
    }
    // After print pipeline, need to check exit status to implement pipeline feature.
    let mut result = Ok(());
    for one_exit in exit_status.into_iter().rev() {
        if let Some((future, span)) = one_exit {
            if let Err(err) = check_exit_status_future_ok(future, span) {
                result = Err(err)
            }
        }
    }
    info!("evaluate {}:{}:{}", file!(), line!(), column!());

    result
}
