use crate::util::{eval_source, print_pipeline};
use log::{info, trace};
use nu_engine::eval_block;
use nu_parser::parse;
use nu_path::absolute_with;
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
        match absolute_with(&path, cwd) {
            Ok(t) => Ok(t),
            Err(err) => Err(IoError::new_internal_with_path(
                err,
                "Invalid path",
                nu_protocol::location!(),
                PathBuf::from(&path),
            )),
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
        let cmdline = format!("nu {path} {}", args.join(" "));
        let mut working_set = StateWorkingSet::new(engine_state);
        let file_id = working_set.add_file("<commandline>".into(), cmdline.as_bytes());
        let span = working_set
            .get_span_for_file(file_id)
            .subspan(3, path.len() + 3)
            .expect("<commandline> to contain script path");
        if let Err(err) = engine_state.merge_delta(working_set.render()) {
            err
        } else {
            IoError::new(err.not_found_as(NotFound::File), span, PathBuf::from(&path)).into()
        }
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

    // we'll need the script name repeatedly; keep both String and bytes
    let script_name = source_filename.to_string_lossy().to_string();
    let script_name_bytes = script_name.as_bytes().to_vec();

    let mut working_set = StateWorkingSet::new(engine_state);
    trace!("parsing file: {file_path_str}");
    let block = parse(&mut working_set, Some(file_path_str), &file, false);

    if let Some(warning) = working_set.parse_warnings.first() {
        report_parse_warning(None, &working_set, warning);
    }

    // If any parse errors were found, report the first error and exit.
    if let Some(err) = working_set.parse_errors.first() {
        report_parse_error(None, &working_set, err);
        std::process::exit(1);
    }

    if let Some(err) = working_set.compile_errors.first() {
        report_compile_error(None, &working_set, err);
        std::process::exit(1);
    }

    // Look for blocks whose name is `main` or begins with `main `; if any are
    // found we:
    // 1. rewrite the signature to use the script's filename,
    // 2. remember that the file contained a `main` command, and
    // 3. later add an alias in the overlay so users can still call `main`.
    let mut file_has_main = false;
    for block in working_set.delta.blocks.iter_mut().map(Arc::make_mut) {
        if block.signature.name == "main" {
            file_has_main = true;
            block.signature.name = script_name.clone();
        } else if block.signature.name.starts_with("main ") {
            file_has_main = true;
            block.signature.name = script_name.clone() + " " + &block.signature.name[5..];
        }
    }

    // If we found a main declaration, alias the overlay entries so that
    // `script.nu` (and `script.nu foo`) resolve just like `main`.
    if file_has_main && let Some(overlay) = working_set.delta.last_overlay_mut() {
        // Collect new entries to avoid mutating while iterating.
        // For "main" → new_name is just the script filename.
        // For "main foo" → name[4..] is " foo" (space included), giving "script.nu foo".
        let mut new_decls = Vec::new();
        for (name, &decl_id) in &overlay.decls {
            if name == b"main" || name.starts_with(b"main ") {
                let mut new_name = script_name_bytes.clone();
                if name.len() > 4 {
                    new_name.extend_from_slice(&name[4..]);
                }
                new_decls.push((new_name, decl_id));
            }
        }
        for (n, id) in new_decls {
            overlay.decls.insert(n, id);
        }

        let mut new_predecls = Vec::new();
        for (name, &decl_id) in &overlay.predecls {
            if name == b"main" || name.starts_with(b"main ") {
                let mut new_name = script_name_bytes.clone();
                if name.len() > 4 {
                    new_name.extend_from_slice(&name[4..]);
                }
                new_predecls.push((new_name, decl_id));
            }
        }
        for (n, id) in new_predecls {
            overlay.predecls.insert(n, id);
        }
    }

    // Merge the changes into the engine state.
    engine_state.merge_delta(working_set.delta)?;

    // Check if the file contains a main command.  We use the script name instead
    // of the literal `main` because the delta (above) may have rewritten the
    // declaration and added an alias.
    let exit_code = if file_has_main && engine_state.find_decl(&script_name_bytes, &[]).is_some() {
        // Evaluate the file, but don't run main yet.
        let pipeline =
            match eval_block::<WithoutDebug>(engine_state, stack, &block, PipelineData::empty())
                .map(|p| p.body)
            {
                Ok(data) => data,
                Err(ShellError::Return { .. }) => {
                    // Allow early return before main is run.
                    return Ok(());
                }
                Err(err) => return Err(err),
            };

        // Print the pipeline output of the last command of the file.
        print_pipeline(engine_state, stack, pipeline, true)?;

        // Invoke the main command with arguments.  Keep using `main` as the
        // internal command name so the parser reliably resolves it; the block's
        // signature was already rewritten to the script filename above, so help
        // messages will show the correct `script.nu`-qualified name.
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
