use nu_engine::{convert_env_values, eval_block};
use nu_parser::parse;
use nu_protocol::{
    debugger::WithoutDebug,
    engine::{EngineState, Stack, StateWorkingSet},
    eval_const::create_nu_constant,
    report_error, PipelineData, ShellError, Spanned, Value, NU_VARIABLE_ID,
};
use std::sync::Arc;

/// Run a command (or commands) given to us by the user
pub fn evaluate_commands(
    commands: &Spanned<String>,
    engine_state: &mut EngineState,
    stack: &mut Stack,
    input: PipelineData,
    table_mode: Option<Value>,
    no_newline: bool,
) -> Result<(), ShellError> {
    // Regenerate the $nu constant to contain the startup time and any other potential updates
    let nu_const = create_nu_constant(engine_state, commands.span)?;
    engine_state.set_variable_const_val(NU_VARIABLE_ID, nu_const);

    // Translate environment variables from Strings to Values
    if let Some(err) = convert_env_values(engine_state, stack) {
        return Err(err);
    }

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

    if let Some(t_mode) = table_mode {
        Arc::make_mut(&mut engine_state.config).table_mode =
            t_mode.coerce_str()?.parse().unwrap_or_default();
    }

    if let Some(status) = data.print(engine_state, stack, no_newline, false)? {
        if status.code() != 0 {
            std::process::exit(status.code())
        }
    }
    Ok(())
}
