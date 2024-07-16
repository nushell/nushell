use crate::util::eval_source;
use log::{info, trace};
use nu_engine::{convert_env_values, eval_block};
use nu_parser::parse;
use nu_path::canonicalize_with;
use nu_protocol::{
    debugger::WithoutDebug,
    engine::{EngineState, Stack, StateWorkingSet},
    report_error, PipelineData, ShellError, Span, Value,
};
use std::sync::Arc;

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
) -> Result<(), ShellError> {
    // Convert environment variables from Strings to Values and store them in the engine state.
    convert_env_values(engine_state, stack)?;

    let cwd = engine_state.cwd_as_string(Some(stack))?;

    let file_path =
        canonicalize_with(&path, cwd).map_err(|err| ShellError::FileNotFoundCustom {
            msg: format!("Could not access file '{path}': {err}"),
            span: Span::unknown(),
        })?;

    let file_path_str = file_path
        .to_str()
        .ok_or_else(|| ShellError::NonUtf8Custom {
            msg: format!(
                "Input file name '{}' is not valid UTF8",
                file_path.to_string_lossy()
            ),
            span: Span::unknown(),
        })?;

    let file = std::fs::read(&file_path).map_err(|err| ShellError::FileNotFoundCustom {
        msg: format!("Could not read file '{file_path_str}': {err}"),
        span: Span::unknown(),
    })?;
    engine_state.file = Some(file_path.clone());

    let parent = file_path
        .parent()
        .ok_or_else(|| ShellError::FileNotFoundCustom {
            msg: format!("The file path '{file_path_str}' does not have a parent"),
            span: Span::unknown(),
        })?;

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

    if let Some(warning) = working_set.parse_warnings.first() {
        report_error(&working_set, warning);
    }

    // If any parse errors were found, report the first error and exit.
    if let Some(err) = working_set.parse_errors.first() {
        report_error(&working_set, err);
        std::process::exit(1);
    }

    if let Some(err) = working_set.compile_errors.first() {
        report_error(&working_set, err);
        // Not a fatal error, for now
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
    engine_state.merge_delta(working_set.delta)?;

    // Check if the file contains a main command.
    let exit_code = if engine_state.find_decl(b"main", &[]).is_some() {
        // Evaluate the file, but don't run main yet.
        let pipeline =
            match eval_block::<WithoutDebug>(engine_state, stack, &block, PipelineData::empty()) {
                Ok(data) => data,
                Err(ShellError::Return { .. }) => {
                    // Allow early return before main is run.
                    return Ok(());
                }
                Err(err) => return Err(err),
            };

        // Print the pipeline output of the last command of the file.
        if let Some(status) = pipeline.print(engine_state, stack, true, false)? {
            if status.code() != 0 {
                std::process::exit(status.code())
            }
        }

        // Invoke the main command with arguments.
        // Arguments with whitespaces are quoted, thus can be safely concatenated by whitespace.
        let args = format!("main {}", args.join(" "));
        eval_source(
            engine_state,
            stack,
            args.as_bytes(),
            "<commandline>",
            input,
            true,
        )
    } else {
        eval_source(engine_state, stack, &file, file_path_str, input, true)
    };

    if exit_code != 0 {
        std::process::exit(exit_code)
    }

    info!("evaluate {}:{}:{}", file!(), line!(), column!());

    Ok(())
}
