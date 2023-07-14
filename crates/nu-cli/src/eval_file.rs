use crate::util::eval_source;
use log::info;
use log::trace;
use miette::{IntoDiagnostic, Result};
use nu_engine::{convert_env_values, current_dir};
use nu_parser::parse;
use nu_path::canonicalize_with;
use nu_protocol::report_error;
use nu_protocol::{
    ast::Call,
    engine::{EngineState, Stack, StateWorkingSet},
    Config, PipelineData, ShellError, Span, Value,
};
use nu_utils::stdout_write_all_and_flush;

/// Main function used when a file path is found as argument for nu
pub fn evaluate_file(
    path: String,
    args: &[String],
    engine_state: &mut EngineState,
    stack: &mut Stack,
    input: PipelineData,
) -> Result<()> {
    // Translate environment variables from Strings to Values
    if let Some(e) = convert_env_values(engine_state, stack) {
        let working_set = StateWorkingSet::new(engine_state);
        report_error(&working_set, &e);
        std::process::exit(1);
    }

    let cwd = current_dir(engine_state, stack)?;

    let file_path = canonicalize_with(&path, cwd).unwrap_or_else(|e| {
        let working_set = StateWorkingSet::new(engine_state);
        report_error(
            &working_set,
            &ShellError::FileNotFoundCustom(
                format!("Could not access file '{}': {:?}", path, e.to_string()),
                Span::unknown(),
            ),
        );
        std::process::exit(1);
    });

    let file_path_str = file_path.to_str().unwrap_or_else(|| {
        let working_set = StateWorkingSet::new(engine_state);
        report_error(
            &working_set,
            &ShellError::NonUtf8Custom(
                format!(
                    "Input file name '{}' is not valid UTF8",
                    file_path.to_string_lossy()
                ),
                Span::unknown(),
            ),
        );
        std::process::exit(1);
    });

    let file = std::fs::read(&file_path)
        .into_diagnostic()
        .unwrap_or_else(|e| {
            let working_set = StateWorkingSet::new(engine_state);
            report_error(
                &working_set,
                &ShellError::FileNotFoundCustom(
                    format!(
                        "Could not read file '{}': {:?}",
                        file_path_str,
                        e.to_string()
                    ),
                    Span::unknown(),
                ),
            );
            std::process::exit(1);
        });

    engine_state.start_in_file(Some(file_path_str));

    let parent = file_path.parent().unwrap_or_else(|| {
        let working_set = StateWorkingSet::new(engine_state);
        report_error(
            &working_set,
            &ShellError::FileNotFoundCustom(
                format!("The file path '{file_path_str}' does not have a parent"),
                Span::unknown(),
            ),
        );
        std::process::exit(1);
    });

    stack.add_env_var(
        "FILE_PWD".to_string(),
        Value::string(parent.to_string_lossy(), Span::unknown()),
    );
    stack.add_env_var(
        "CURRENT_FILE".to_string(),
        Value::string(file_path.to_string_lossy(), Span::unknown()),
    );

    let mut working_set = StateWorkingSet::new(engine_state);
    trace!("parsing file: {}", file_path_str);
    let _ = parse(&mut working_set, Some(file_path_str), &file, false);

    if working_set.find_decl(b"main").is_some() {
        let args = format!("main {}", args.join(" "));

        if !eval_source(
            engine_state,
            stack,
            &file,
            file_path_str,
            PipelineData::empty(),
            true,
        ) {
            std::process::exit(1);
        }
        if !eval_source(
            engine_state,
            stack,
            args.as_bytes(),
            "<commandline>",
            input,
            true,
        ) {
            std::process::exit(1);
        }
    } else if !eval_source(engine_state, stack, &file, file_path_str, input, true) {
        std::process::exit(1);
    }

    info!("evaluate {}:{}:{}", file!(), line!(), column!());

    Ok(())
}

pub(crate) fn print_table_or_error(
    engine_state: &mut EngineState,
    stack: &mut Stack,
    mut pipeline_data: PipelineData,
    config: &mut Config,
) -> Option<i64> {
    let exit_code = match &mut pipeline_data {
        PipelineData::ExternalStream { exit_code, .. } => exit_code.take(),
        _ => None,
    };

    // Change the engine_state config to use the passed in configuration
    engine_state.set_config(config);

    if let PipelineData::Value(Value::Error { error }, ..) = &pipeline_data {
        let working_set = StateWorkingSet::new(engine_state);
        report_error(&working_set, &**error);
        std::process::exit(1);
    }

    if let Some(decl_id) = engine_state.find_decl("table".as_bytes(), &[]) {
        let command = engine_state.get_decl(decl_id);
        if command.get_block_id().is_some() {
            print_or_exit(pipeline_data, engine_state, config);
        } else {
            // The final call on table command, it's ok to set redirect_output to false.
            let mut call = Call::new(Span::new(0, 0));
            call.redirect_stdout = false;
            let table = command.run(engine_state, stack, &call, pipeline_data);

            match table {
                Ok(table) => {
                    print_or_exit(table, engine_state, config);
                }
                Err(error) => {
                    let working_set = StateWorkingSet::new(engine_state);
                    report_error(&working_set, &error);
                    std::process::exit(1);
                }
            }
        }
    } else {
        print_or_exit(pipeline_data, engine_state, config);
    }

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

fn print_or_exit(pipeline_data: PipelineData, engine_state: &mut EngineState, config: &Config) {
    for item in pipeline_data {
        if let Value::Error { error } = item {
            let working_set = StateWorkingSet::new(engine_state);

            report_error(&working_set, &*error);

            std::process::exit(1);
        }

        let out = item.into_string("\n", config) + "\n";
        let _ = stdout_write_all_and_flush(out).map_err(|err| eprintln!("{err}"));
    }
}
