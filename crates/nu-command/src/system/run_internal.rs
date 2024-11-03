use log::info;
use nu_engine::command_prelude::*;
use nu_engine::{convert_env_values, eval_block};
use nu_parser::parse;
use nu_protocol::{
    cli_error::report_compile_error,
    debugger::WithoutDebug,
    engine::{EngineState, Stack, StateWorkingSet},
    report_parse_error, report_parse_warning, IntoValue, PipelineData, ShellError, Spanned, Value,
};
use std::sync::Arc;

#[derive(Clone)]
pub struct Internal;

impl Command for Internal {
    fn name(&self) -> &str {
        "run-internal"
    }

    fn description(&self) -> &str {
        "Runs internal command."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build(self.name())
            .input_output_types(vec![(Type::Any, Type::Any)])
            .required("command", SyntaxShape::String, "Internal command to run.")
            .category(Category::System)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Run an internal command",
                example: r#"run-internal "ls""#,
                result: None,
            },
            Example {
                description: "Run a pipeline",
                example: r#"run-internal "print (ls | first 5);print (ps | first 5)"#,
                result: None,
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let config = engine_state.get_config();
        let table_mode = config.table.mode.into_value(call.head);
        let error_style = config.error_style.into_value(call.head);
        let no_newline = false;
        let commands: Spanned<String> = call.req(engine_state, stack, 0)?;
        // let _ = evaluate_commands(
        //     &commands,
        //     &mut engine_state.clone(),
        //     &mut stack.clone(),
        //     input,
        //     EvaluateCommandsOpts {
        //         table_mode: Some(table_mode),
        //         error_style: Some(error_style),
        //         no_newline,
        //     },
        // );
        // Ok(PipelineData::Empty)
        evaluate_commands(
            &commands,
            &mut engine_state.clone(),
            &mut stack.clone(),
            input,
            EvaluateCommandsOpts {
                table_mode: Some(table_mode),
                error_style: Some(error_style),
                no_newline,
            },
        )
    }
}

// This code is ripped off from nu-cli. It's duplicated here because I didn't
// want to add a dependency on nu-cli in nu-command.
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
) -> Result<PipelineData, ShellError> {
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
            // Not a fatal error, for now
        }

        (output, working_set.render())
    };

    // Update permanent state
    engine_state.merge_delta(delta)?;

    // Run the block
    // let pipeline = eval_block::<WithoutDebug>(engine_state, stack, &block, input)?;
    eval_block::<WithoutDebug>(engine_state, stack, &block, input)

    // if let PipelineData::Value(Value::Error { error, .. }, ..) = pipeline {
    //     return Err(*error);
    // }

    // if let Some(t_mode) = table_mode {
    //     Arc::make_mut(&mut engine_state.config).table.mode =
    //         t_mode.coerce_str()?.parse().unwrap_or_default();
    // }

    // pipeline.print(engine_state, stack, no_newline, false)?;

    // info!("evaluate {}:{}:{}", file!(), line!(), column!());

    // Ok(())
}

#[cfg(test)]
mod test {
    // use super::*;
    // use nu_test_support::{fs::Stub, playground::Playground};

    // #[test]
    // fn test_some_test() {
    // }

    // #[test]
    // fn test_some_other_test() {
    // }
}
