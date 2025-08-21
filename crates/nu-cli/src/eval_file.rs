use crate::util::{eval_source, print_pipeline};
use log::{info, trace};
use nu_engine::eval_block;
use nu_parser::parse;
use nu_path::canonicalize_with;
use nu_protocol::{
    PipelineData, ShellError, Span, Value,
    debugger::WithoutDebug,
    engine::{EngineState, Stack, StateWorkingSet},
    report_error::report_compile_error,
    report_parse_error, report_parse_warning,
    shell_error::io::*,
};
use std::{path::PathBuf, sync::Arc};

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
    let cwd = engine_state.cwd_as_string(Some(stack))?;

    let file_path = {
        match canonicalize_with(&path, cwd) {
            Ok(t) => Ok(t),
            Err(err) => {
                let cmdline = format!("nu {path} {}", args.join(" "));
                let mut working_set = StateWorkingSet::new(engine_state);
                let file_id = working_set.add_file("<commandline>".into(), cmdline.as_bytes());
                let span = working_set
                    .get_span_for_file(file_id)
                    .subspan(3, path.len() + 3)
                    .expect("<commandline> to contain script path");
                engine_state.merge_delta(working_set.render())?;
                let e = IoError::new(err.not_found_as(NotFound::File), span, PathBuf::from(&path));
                Err(e)
            }
        }
    }?;

    let file_path_str = file_path
        .to_str()
        .ok_or_else(|| ShellError::NonUtf8Custom {
            msg: format!(
                "Input file name '{}' is not valid UTF8",
                file_path.to_string_lossy()
            ),
            span: Span::unknown(),
        })?;

    let file = std::fs::read(&file_path).map_err(|err| {
        IoError::new_internal_with_path(
            err.not_found_as(NotFound::File),
            "Could not read file",
            nu_protocol::location!(),
            file_path.clone(),
        )
    })?;
    engine_state.file = Some(file_path.clone());

    let parent = file_path.parent().ok_or_else(|| {
        IoError::new_internal_with_path(
            ErrorKind::DirectoryNotFound,
            "The file path does not have a parent",
            nu_protocol::location!(),
            file_path.clone(),
        )
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
    trace!("parsing file: {file_path_str}");
    let block = parse(&mut working_set, Some(file_path_str), &file, false);

    if let Some(warning) = working_set.parse_warnings.first() {
        report_parse_warning(&working_set, warning);
    }

    // If any parse errors were found, report the first error and exit.
    if let Some(err) = working_set.parse_errors.first() {
        report_parse_error(&working_set, err);
        std::process::exit(1);
    }

    if let Some(err) = working_set.compile_errors.first() {
        report_compile_error(&working_set, err);
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
        print_pipeline(engine_state, stack, pipeline, true)?;

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
        std::process::exit(exit_code);
    }

    info!("evaluate {}:{}:{}", file!(), line!(), column!());

    Ok(())
}
