use crate::util::eval_source;
use log::info;
use log::trace;
use miette::{IntoDiagnostic, Result};
use nu_engine::eval_block;
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

    engine_state.start_in_file(Some(file_path_str));

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
        .expect("internal error: script missing filename");

    let mut working_set = StateWorkingSet::new(engine_state);
    trace!("parsing file: {}", file_path_str);
    let block = parse(&mut working_set, Some(file_path_str), &file, false);

    if let Some(err) = working_set.parse_errors.first() {
        report_error(&working_set, err);
        std::process::exit(1);
    }

    for block in &mut working_set.delta.blocks {
        if block.signature.name == "main" {
            block.signature.name = source_filename.to_string_lossy().to_string();
        } else if block.signature.name.starts_with("main ") {
            block.signature.name =
                source_filename.to_string_lossy().to_string() + " " + &block.signature.name[5..];
        }
    }

    let _ = engine_state.merge_delta(working_set.delta);

    if engine_state.find_decl(b"main", &[]).is_some() {
        let args = format!("main {}", args.join(" "));

        let pipeline_data = eval_block(
            engine_state,
            stack,
            &block,
            PipelineData::empty(),
            false,
            false,
        );
        let pipeline_data = match pipeline_data {
            Err(ShellError::Return { .. }) => {
                // allows early exists before `main` is run.
                return Ok(());
            }

            x => x,
        }
        .unwrap_or_else(|e| {
            let working_set = StateWorkingSet::new(engine_state);
            report_error(&working_set, &e);
            std::process::exit(1);
        });

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
    engine_state.set_config(config.clone());

    if let PipelineData::Value(Value::Error { error, .. }, ..) = &pipeline_data {
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
        if let Value::Error { error, .. } = item {
            let working_set = StateWorkingSet::new(engine_state);

            report_error(&working_set, &*error);

            std::process::exit(1);
        }

        let out = item.to_expanded_string("\n", config) + "\n";
        let _ = stdout_write_all_and_flush(out).map_err(|err| eprintln!("{err}"));
    }
}
