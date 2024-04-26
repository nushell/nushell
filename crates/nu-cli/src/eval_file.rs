use crate::util::eval_source;
use log::{info, trace};
use miette::{IntoDiagnostic, Result};
use nu_engine::{convert_env_values, current_dir, eval_block};
use nu_parser::parse;
use nu_path::canonicalize_with;
use nu_protocol::{
    debugger::WithoutDebug,
    engine::{EngineState, Stack, StateWorkingSet},
    report_error, Config, PipelineData, ShellError, Span, Value,
};
use std::{io::Write, sync::Arc};

/// Entry point for evaluating a file.
///
/// If the file contains a main command, it is invoked with `args` and the pipeline data from `input`;
/// otherwise, the pipeline data is forwarded to the first command in the file, and `args` are ignored.
pub fn evaluate_file(
    path: String,
    args: &[String],
    engine_state: &mut EngineState,
    stack: &mut Stack,
    input: PipelineData,
) -> Result<()> {
    // Convert environment variables from Strings to Values and store them in the engine state.
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
            &ShellError::FileNotFoundCustom {
                msg: format!("Could not access file '{}': {:?}", path, e.to_string()),
                span: Span::unknown(),
            },
        );
        std::process::exit(1);
    });

    let file_path_str = file_path.to_str().unwrap_or_else(|| {
        let working_set = StateWorkingSet::new(engine_state);
        report_error(
            &working_set,
            &ShellError::NonUtf8Custom {
                msg: format!(
                    "Input file name '{}' is not valid UTF8",
                    file_path.to_string_lossy()
                ),
                span: Span::unknown(),
            },
        );
        std::process::exit(1);
    });

    let file = std::fs::read(&file_path)
        .into_diagnostic()
        .unwrap_or_else(|e| {
            let working_set = StateWorkingSet::new(engine_state);
            report_error(
                &working_set,
                &ShellError::FileNotFoundCustom {
                    msg: format!(
                        "Could not read file '{}': {:?}",
                        file_path_str,
                        e.to_string()
                    ),
                    span: Span::unknown(),
                },
            );
            std::process::exit(1);
        });
    engine_state.file = Some(file_path.clone());

    let parent = file_path.parent().unwrap_or_else(|| {
        let working_set = StateWorkingSet::new(engine_state);
        report_error(
            &working_set,
            &ShellError::FileNotFoundCustom {
                msg: format!("The file path '{file_path_str}' does not have a parent"),
                span: Span::unknown(),
            },
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
    stack.add_env_var(
        "PROCESS_PATH".to_string(),
        Value::string(path, Span::unknown()),
    );

    let source_filename = file_path
        .file_name()
        .expect("internal error: missing filename");

    let mut working_set = StateWorkingSet::new(engine_state);
    trace!("parsing file: {}", file_path_str);
    let block = parse(&mut working_set, Some(file_path_str), &file, false);

    // If any parse errors were found, report the first error and exit.
    if let Some(err) = working_set.parse_errors.first() {
        report_error(&working_set, err);
        std::process::exit(1);
    }

    // Look for blocks whose name starts with "main" and replace it with the filename.
    for block in working_set.delta.blocks.iter_mut().map(Arc::make_mut) {
        if block.signature.name == "main" {
            block.signature.name = source_filename.to_string_lossy().to_string();
        } else if block.signature.name.starts_with("main ") {
            block.signature.name =
                source_filename.to_string_lossy().to_string() + " " + &block.signature.name[5..];
        }
    }

    // Merge the changes into the engine state.
    engine_state
        .merge_delta(working_set.delta)
        .expect("merging delta into engine_state should succeed");

    // Check if the file contains a main command.
    if engine_state.find_decl(b"main", &[]).is_some() {
        // Evaluate the file, but don't run main yet.
        let pipeline_data =
            eval_block::<WithoutDebug>(engine_state, stack, &block, PipelineData::empty());
        let pipeline_data = match pipeline_data {
            Err(ShellError::Return { .. }) => {
                // Allow early return before main is run.
                return Ok(());
            }
            x => x,
        }
        .unwrap_or_else(|e| {
            let working_set = StateWorkingSet::new(engine_state);
            report_error(&working_set, &e);
            std::process::exit(1);
        });

        // Print the pipeline output of the file.
        // The pipeline output of a file is the pipeline output of its last command.
        let result = pipeline_data.print(engine_state, stack, true, false);
        match result {
            Err(err) => {
                let working_set = StateWorkingSet::new(engine_state);
                report_error(&working_set, &err);
                std::process::exit(1);
            }
            Ok(exit_code) => {
                if exit_code != 0 {
                    std::process::exit(exit_code as i32);
                }
            }
        }

        // Invoke the main command with arguments.
        // Arguments with whitespaces are quoted, thus can be safely concatenated by whitespace.
        let args = format!("main {}", args.join(" "));
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
