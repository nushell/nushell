use crate::util::{eval_source, evaluate_block_with_exit_code};
use log::{info, trace};
use nu_engine::{convert_env_values, eval_block};
use nu_parser::{find_longest_command, lex, lite_parse, parse, parse_internal_call};
use nu_path::canonicalize_with;
use nu_protocol::ast::{Block, Expr, Expression, Pipeline};
use nu_protocol::{
    debugger::WithoutDebug,
    engine::{EngineState, Stack, StateWorkingSet},
    report_error, PipelineData, ShellError, Signature, Span, Value,
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

    let mut working_set = StateWorkingSet::new(engine_state);
    let source = format!("{} {}", source_filename.to_string_lossy(), args.join(" "));
    let source = source.as_bytes();
    let file_id = working_set.add_file("<commandline>".to_string(), source);
    let file_span = working_set.get_span_for_file(file_id);
    let (tokens, err) = lex(source, file_span.start, &[], &[], false);
    if let Some(err) = err {
        working_set.error(err)
    }
    let (lite_block, err) = lite_parse(tokens.as_slice());
    if let Some(err) = err {
        working_set.error(err);
    }
    let lite_command = &lite_block.block[0].commands[0];

    let exit_code = if let Some((decl_id, command_len)) =
        find_longest_command(&working_set, b"main", &lite_command.parts[1..])
    {
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

        let parsed_call = parse_internal_call(
            &mut working_set,
            Span::concat(&lite_command.parts[..command_len + 1]),
            &lite_command.parts[(command_len + 1)..],
            decl_id,
        );

        let expression = Expression::new(
            &mut working_set,
            Expr::Call(parsed_call.call),
            Span::concat(lite_command.parts.as_slice()),
            parsed_call.output,
        );

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

        let mut main_call = Block {
            signature: Box::new(Signature::new("")),
            pipelines: vec![Pipeline::from_vec(vec![expression])],
            captures: vec![],
            redirect_env: true,
            ir_block: None,
            span: Some(file_span),
        };

        match nu_engine::compile(&working_set, &main_call) {
            Ok(ir_block) => {
                main_call.ir_block = Some(ir_block);
            }
            Err(err) => working_set.compile_errors.push(err),
        }

        engine_state.merge_delta(working_set.delta)?;

        evaluate_block_with_exit_code(
            engine_state,
            stack,
            Arc::new(main_call),
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
