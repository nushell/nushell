#![allow(clippy::byte_char_slices)]

use nu_cmd_base::hook::eval_hook;
use nu_engine::{eval_block, eval_block_with_early_return};
use nu_parser::{Token, TokenContents, lex, parse, unescape_unquote_string};
use nu_protocol::{
    ByteStream, ByteStreamSource, ListStream, PipelineData, PipelineMetadata, ShellError, Signals,
    Span, Value,
    ast::Block,
    block_is_bare_last_result_with,
    debugger::WithoutDebug,
    engine::{EngineState, Stack, StateWorkingSet},
    process::check_exit_status_future,
    report_error::report_compile_error,
    report_parse_error, report_parse_warning, report_shell_error,
    shell_error::{generic::GenericError, io::IoError},
    truncate_value_to_budget, value_from_bytes,
};
#[cfg(windows)]
use nu_utils::enable_vt_processing;
use nu_utils::time::Instant;
use nu_utils::{escape_quote_string, perf};
use std::path::Path;

// This will collect environment variables from std::env and adds them to a stack.
//
// In order to ensure the values have spans, it first creates a dummy file, writes the collected
// env vars into it (in a "NAME"="value" format, quite similar to the output of the Unix 'env'
// tool), then uses the file to get the spans. The file stays in memory, no filesystem IO is done.
//
// The "PWD" env value will be forced to `init_cwd`.
// The reason to use `init_cwd`:
//
// While gathering parent env vars, the parent `PWD` may not be the same as `current working directory`.
// Consider to the following command as the case (assume we execute command inside `/tmp`):
//
//     tmux split-window -v -c "#{pane_current_path}"
//
// Here nu execute external command `tmux`, and tmux starts a new `nushell`, with `init_cwd` value "#{pane_current_path}".
// But at the same time `PWD` still remains to be `/tmp`.
//
// In this scenario, the new `nushell`'s PWD should be "#{pane_current_path}" rather init_cwd.
pub fn gather_parent_env_vars(engine_state: &mut EngineState, init_cwd: &Path) {
    gather_env_vars(std::env::vars(), engine_state, init_cwd);
}

fn gather_env_vars(
    vars: impl Iterator<Item = (String, String)>,
    engine_state: &mut EngineState,
    init_cwd: &Path,
) {
    fn report_capture_error(engine_state: &EngineState, env_str: &str, msg: &str) {
        report_shell_error(
            None,
            engine_state,
            &ShellError::Generic(
                GenericError::new_internal(
                    format!("Environment variable was not captured: {env_str}"),
                    "",
                )
                .with_help(msg.to_string()),
            ),
        );
    }

    fn put_env_to_fake_file(name: &str, val: &str, fake_env_file: &mut String) {
        fake_env_file.push_str(&escape_quote_string(name));
        fake_env_file.push('=');
        fake_env_file.push_str(&escape_quote_string(val));
        fake_env_file.push('\n');
    }

    let mut fake_env_file = String::new();
    // Write all the env vars into a fake file
    for (name, val) in vars {
        put_env_to_fake_file(&name, &val, &mut fake_env_file);
    }

    match init_cwd.to_str() {
        Some(cwd) => {
            put_env_to_fake_file("PWD", cwd, &mut fake_env_file);
        }
        None => {
            // Could not capture current working directory
            report_shell_error(
                None,
                engine_state,
                &ShellError::Generic(
                    GenericError::new_internal("Current directory is not a valid utf-8 path", "")
                        .with_help(format!(
                            "Retrieving current directory failed: {init_cwd:?} not a valid utf-8 path"
                        )),
                ),
            );
        }
    }

    // Lex the fake file, assign spans to all environment variables and add them
    // to stack
    let span_offset = engine_state.next_span_start();

    engine_state.add_file(
        "Host Environment Variables".into(),
        fake_env_file.as_bytes().into(),
    );

    let (tokens, _) = lex(fake_env_file.as_bytes(), span_offset, &[], &[], true);

    for token in tokens {
        if let Token {
            contents: TokenContents::Item,
            span: full_span,
        } = token
        {
            let contents = engine_state.get_span_contents(full_span);
            let (parts, _) = lex(contents, full_span.start, &[], &[b'='], true);

            let name = if let Some(Token {
                contents: TokenContents::Item,
                span,
            }) = parts.first()
            {
                let mut working_set = StateWorkingSet::new(engine_state);
                let bytes = working_set.get_span_contents(*span);

                if bytes.len() < 2 {
                    report_capture_error(
                        engine_state,
                        &String::from_utf8_lossy(contents),
                        "Got empty name.",
                    );

                    continue;
                }

                let (bytes, err) = unescape_unquote_string(bytes, *span);
                if let Some(err) = err {
                    working_set.error(err);
                }

                if !working_set.parse_errors.is_empty() {
                    report_capture_error(
                        engine_state,
                        &String::from_utf8_lossy(contents),
                        "Got unparsable name.",
                    );

                    continue;
                }

                bytes
            } else {
                report_capture_error(
                    engine_state,
                    &String::from_utf8_lossy(contents),
                    "Got empty name.",
                );

                continue;
            };

            let value = if let Some(Token {
                contents: TokenContents::Item,
                span,
            }) = parts.get(2)
            {
                let mut working_set = StateWorkingSet::new(engine_state);
                let bytes = working_set.get_span_contents(*span);

                if bytes.len() < 2 {
                    report_capture_error(
                        engine_state,
                        &String::from_utf8_lossy(contents),
                        "Got empty value.",
                    );

                    continue;
                }

                let (bytes, err) = unescape_unquote_string(bytes, *span);
                if let Some(err) = err {
                    working_set.error(err);
                }

                if !working_set.parse_errors.is_empty() {
                    report_capture_error(
                        engine_state,
                        &String::from_utf8_lossy(contents),
                        "Got unparsable value.",
                    );

                    continue;
                }

                Value::string(bytes, *span)
            } else {
                report_capture_error(
                    engine_state,
                    &String::from_utf8_lossy(contents),
                    "Got empty value.",
                );

                continue;
            };

            // stack.add_env_var(name, value);
            engine_state.add_env_var(name, value);
        }
    }
}

/// Print a pipeline with formatting applied based on display_output hook.
///
/// This function should be preferred when printing values resulting from a completed evaluation.
/// For values printed as part of a command's execution, such as values printed by the `print` command,
/// the `PipelineData::print_table` function should be preferred instead as it is not config-dependent.
///
/// `no_newline` controls if we need to attach newline character to output.
pub fn print_pipeline(
    engine_state: &mut EngineState,
    stack: &mut Stack,
    pipeline: PipelineData,
    no_newline: bool,
) -> Result<(), ShellError> {
    let to_stderr = engine_state.is_mcp || engine_state.is_lsp;

    if let Some(hook) = stack.get_config(engine_state).hooks.display_output.clone() {
        let pipeline = eval_hook(
            engine_state,
            stack,
            Some(pipeline),
            vec![],
            &hook,
            "display_output",
        )?;
        pipeline.print_raw(engine_state, no_newline, to_stderr)
    } else {
        // if display_output isn't set, we should still prefer to print with some formatting
        pipeline.print_table(engine_state, stack, no_newline, to_stderr)
    }
}

pub fn eval_source(
    engine_state: &mut EngineState,
    stack: &mut Stack,
    source: &[u8],
    fname: &str,
    input: PipelineData,
    allow_return: bool,
) -> i32 {
    let start_time = Instant::now();

    let exit_code = match evaluate_source(engine_state, stack, source, fname, input, allow_return) {
        Ok(failed) => {
            let code = failed.into();
            // No call span available in eval_source — this wraps generic source evaluation
            stack.set_last_exit_code(code, Span::unknown());
            code
        }
        Err(err) => map_eval_error_to_exit_code(engine_state, stack, err),
    };

    finish_eval_source(engine_state, fname, start_time, exit_code)
}

/// Evaluate an already-parsed block with the same print / exit-code behavior as [`eval_source`].
///
/// Used by file evaluation so the file is not re-parsed (re-parsing can reuse stale sourced
/// blocks with old VarIds; see https://github.com/nushell/nushell/issues/18515).
pub fn eval_parsed_block_source(
    engine_state: &mut EngineState,
    stack: &mut Stack,
    block: &Block,
    fname: &str,
    input: PipelineData,
    allow_return: bool,
) -> i32 {
    let start_time = Instant::now();

    let exit_code = match evaluate_parsed_block(engine_state, stack, block, input, allow_return) {
        Ok(failed) => {
            let code = failed.into();
            stack.set_last_exit_code(code, Span::unknown());
            code
        }
        Err(err) => map_eval_error_to_exit_code(engine_state, stack, err),
    };

    finish_eval_source(engine_state, fname, start_time, exit_code)
}

fn map_eval_error_to_exit_code(
    engine_state: &EngineState,
    stack: &mut Stack,
    err: ShellError,
) -> i32 {
    if let ShellError::Exit { code, .. } = &err {
        std::process::exit(*code)
    }
    report_shell_error(Some(stack), engine_state, &err);
    let code = err.exit_code();
    stack.set_last_error(&err);
    code.unwrap_or(0)
}

fn finish_eval_source(
    engine_state: &EngineState,
    fname: &str,
    start_time: Instant,
    exit_code: i32,
) -> i32 {
    // reset vt processing, aka ansi because illbehaved externals can break it
    #[cfg(windows)]
    {
        let _ = enable_vt_processing();
    }

    perf!(
        &format!("eval_source {}", fname),
        start_time,
        engine_state
            .get_config()
            .use_ansi_coloring
            .get(engine_state)
    );

    exit_code
}

pub(crate) fn evaluate_source(
    engine_state: &mut EngineState,
    stack: &mut Stack,
    source: &[u8],
    fname: &str,
    input: PipelineData,
    allow_return: bool,
) -> Result<bool, ShellError> {
    let (block, delta) = {
        let mut working_set = StateWorkingSet::new(engine_state);
        let output = parse(
            &mut working_set,
            Some(fname), // format!("repl_entry #{}", entry_num)
            source,
            false,
        );
        if let Some(warning) = working_set.parse_warnings.first() {
            report_parse_warning(Some(stack), &working_set, warning);
        }

        if let Some(err) = working_set.parse_errors.first() {
            report_parse_error(Some(stack), &working_set, err);
            return Ok(true);
        }

        if let Some(err) = working_set.compile_errors.first() {
            report_compile_error(Some(stack), &working_set, err);
            return Ok(true);
        }

        (output, working_set.render())
    };

    engine_state.merge_delta(delta)?;

    evaluate_parsed_block(engine_state, stack, &block, input, allow_return)
}

/// Evaluate a parsed block: run it, apply variable deletions, optionally store interactive
/// last-result (`$ans.last`), print output, flush last-result truncation warnings, and handle
/// optional pipefail.
pub(crate) fn evaluate_parsed_block(
    engine_state: &mut EngineState,
    stack: &mut Stack,
    block: &Block,
    input: PipelineData,
    allow_return: bool,
) -> Result<bool, ShellError> {
    let pipeline = if allow_return {
        eval_block_with_early_return::<WithoutDebug>(engine_state, stack, block, input)
    } else {
        eval_block::<WithoutDebug>(engine_state, stack, block, input)
    }?;
    let mut pipeline_data = pipeline.body;

    // Update engine_state with deleted variables
    for var_id in &stack.deletions {
        if let Some(active_id) = engine_state.scope.active_overlays.last()
            && let Some((_, overlay)) = engine_state.scope.overlays.get_mut((*active_id).get())
        {
            overlay.vars.retain(|_, v| *v != *var_id);
        }
    }
    stack.deletions.clear();

    // Capture interactive last-result before display_output transforms/prints the value.
    // Only for true REPL user lines (`capture_repl_last_result`), not config/env/banner.
    if engine_state.is_interactive && engine_state.capture_repl_last_result {
        pipeline_data = maybe_store_last_result(engine_state, stack, block, pipeline_data);
    }

    let no_newline = matches!(&pipeline_data, &PipelineData::ByteStream(..));
    print_pipeline(engine_state, stack, pipeline_data, no_newline)?;

    // Truncation warning for `$ans` is deferred until after display so the data
    // is not scrolled off the screen by the warning.
    stack.flush_last_result_truncation_warning(engine_state, Span::unknown());

    let pipefail = nu_experimental::PIPE_FAIL.get();
    if !pipefail {
        return Ok(false);
    }
    // After print pipeline, need to check exit status to implement pipeline feature.
    check_exit_status_future(pipeline.exit).map(|_| false)
}

/// Store the successful interactive pipeline result as last-result when appropriate.
///
/// Skips bare last-result retrievals. May wrap streams so a prefix is retained for
/// the last-result variable without fully collecting the stream just for storage.
fn maybe_store_last_result(
    engine_state: &EngineState,
    stack: &mut Stack,
    block: &Block,
    pipeline_data: PipelineData,
) -> PipelineData {
    let budget = stack.get_config(engine_state).max_last_result_size_bytes();

    // Bare `$ans` / `$ans.*` cell-paths (rename via LAST_RESULT_VAR_NAME) must not re-store `.last`.
    let mut get_block = |id| engine_state.get_block(id).as_ref();
    if block_is_bare_last_result_with(block, &mut get_block) {
        return pipeline_data;
    }

    if budget == 0 {
        // Budget 0: drop `.last`; snapshot still sets exit_code/duration/command.
        stack.clear_last_result_payload();
        return pipeline_data;
    }

    let signals = engine_state.signals().clone();

    match pipeline_data {
        PipelineData::Empty => {
            stack.set_last_result(Value::nothing(Span::unknown()), None, budget);
            PipelineData::Empty
        }
        PipelineData::Value(value, metadata) => {
            stack.set_last_result(value.clone(), metadata.clone(), budget);
            PipelineData::Value(value, metadata)
        }
        PipelineData::ListStream(stream, metadata) => {
            store_list_stream_prefix(stack, stream, metadata, budget, signals)
        }
        PipelineData::ByteStream(stream, metadata) => {
            store_byte_stream_prefix(stack, stream, metadata, budget)
        }
    }
}

/// Tee a list stream: accumulate `$ans.last` under budget while rebuilding a print stream.
///
/// Under budget with a fully drained stream, avoids double-cloning into a separate print
/// buffer. Overflowing items stop storage (whole rows for tables; optional partial last
/// scalar for non-tables via [`truncate_value_to_budget`]).
fn store_list_stream_prefix(
    stack: &mut Stack,
    stream: ListStream,
    metadata: Option<PipelineMetadata>,
    budget: usize,
    signals: Signals,
) -> PipelineData {
    let span = stream.span();
    let mut kept: Vec<Value> = Vec::new();
    let mut used = Value::list(vec![], span).memory_size();
    let mut truncated = false;
    let mut overflow_item: Option<Value> = None;

    let mut iter = stream.into_iter();
    for item in iter.by_ref() {
        let item_size = item.memory_size();
        if used.saturating_add(item_size) <= budget {
            used += item_size;
            kept.push(item);
            continue;
        }

        // Item does not fit whole.
        truncated = true;
        let table_like = matches!(item, Value::Record { .. })
            && (kept.is_empty() || kept.iter().all(|v| matches!(v, Value::Record { .. })));

        if table_like {
            // Prefer whole rows so table-like `$ans.last` still renders with columns.
            overflow_item = Some(item);
            break;
        }

        // Non-table: allow a partial last scalar/string/binary/list like Value path.
        let remaining = budget.saturating_sub(used);
        if remaining > 0 {
            let (partial, _) = truncate_value_to_budget(item.clone(), remaining);
            if !matches!(partial, Value::Nothing { .. })
                && used.saturating_add(partial.memory_size()) <= budget
            {
                kept.push(partial);
            }
        }
        overflow_item = Some(item);
        break;
    }

    let stored = Value::list(kept.clone(), span);
    let (stored, more_trunc) = if stored.memory_size() > budget {
        truncate_value_to_budget(stored, budget)
    } else {
        (stored, false)
    };
    stack.store_last_result_raw(stored, metadata.clone(), truncated || more_trunc);

    // Print stream: under-budget full drain reuses `kept` without a second buffer of clones.
    let print_iter: Box<dyn Iterator<Item = Value> + Send> = match overflow_item {
        None => Box::new(kept.into_iter()),
        Some(item) => Box::new(kept.into_iter().chain(std::iter::once(item)).chain(iter)),
    };

    PipelineData::ListStream(ListStream::new(print_iter, span, signals), metadata)
}

fn store_byte_stream_prefix(
    stack: &mut Stack,
    stream: ByteStream,
    metadata: Option<PipelineMetadata>,
    budget: usize,
) -> PipelineData {
    let span = stream.span();
    let type_ = stream.type_();
    let signals = stream.signals().clone();
    // Externals (and file-backed streams) trim a single trailing newline when
    // decoded to string, matching [`ByteStream::into_value`].
    let trim_trailing_newline = stream.source().is_external();

    // No capturable bytes (e.g. stdout was null or still inherited/TTY). Do not
    // replace a prior `$ans.last` with empty binary; leave the stream for print/wait.
    // Bare interactive externals keep inherited stdout so TUI tools (`nvim`, `btm`)
    // still attach to the real terminal — capturing them would require a pipe and hang.
    let has_stdout = match stream.source() {
        ByteStreamSource::Read(_) | ByteStreamSource::File(_) => true,
        ByteStreamSource::Child(child) => child.stdout.is_some(),
    };
    if !has_stdout {
        return PipelineData::ByteStream(stream, metadata);
    }

    // Only clear payload after we successfully obtain a reader — `reader()` consumes the stream.
    let Some(mut reader) = stream.reader() else {
        // Defensive: source said stdout exists but reader failed. Prior `$ans` left intact;
        // nothing left to print.
        return PipelineData::Empty;
    };

    // Drop prior `.last` only; keep exit_code/duration until REPL snapshot refreshes them.
    stack.clear_last_result_payload();

    let max_bytes = budget.saturating_sub(std::mem::size_of::<Value>());
    let mut prefix = Vec::new();
    let mut buf = [0u8; 8192];
    let mut truncated = false;

    while prefix.len() < max_bytes {
        // Honor Ctrl-C while filling the budget (large streams can take a while).
        if signals.check(&span).is_err() {
            truncated = true;
            break;
        }
        let to_read = (max_bytes - prefix.len()).min(buf.len());
        match std::io::Read::read(&mut reader, &mut buf[..to_read]) {
            Ok(0) => break,
            Ok(n) => prefix.extend_from_slice(&buf[..n]),
            // EINTR: retry like ByteStream::into_bytes.
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
    }

    // Peek whether more data remains (truncation), unless we already stopped for interrupt.
    let mut trailing: Option<u8> = None;
    if !truncated && prefix.len() >= max_bytes {
        if signals.check(&span).is_err() {
            truncated = true;
        } else {
            match std::io::Read::read(&mut reader, &mut buf[..1]) {
                Ok(0) => {}
                Ok(1..) => {
                    truncated = true;
                    trailing = Some(buf[0]);
                }
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {
                    // Treat as "may have more" without consuming a peek byte.
                    truncated = true;
                }
                _ => {}
            }
        }
    }

    // Decode for `$ans.last` the same way collecting a byte stream does:
    // Binary stays binary; String/Unknown become string when UTF-8, else binary.
    // Display still uses the raw rebuilt byte stream below.
    let stored = value_from_bytes(
        prefix.clone(),
        span,
        type_,
        trim_trailing_newline && !truncated,
    );
    let (stored, more) = if stored.memory_size() > budget {
        truncate_value_to_budget(stored, budget)
    } else {
        (stored, false)
    };
    stack.store_last_result_raw(stored, metadata.clone(), truncated || more);

    // Rebuild stream: stored prefix bytes + optional peek byte + remaining reader.
    let prefix_for_print = prefix;
    let rebuilt = ByteStream::from_result_iter(
        std::iter::once(Ok::<Vec<u8>, ShellError>(prefix_for_print))
            .chain(trailing.map(|b| Ok(vec![b])))
            .chain(std::iter::from_fn(move || {
                let mut chunk = vec![0u8; 8192];
                match std::io::Read::read(&mut reader, &mut chunk) {
                    Ok(0) => None,
                    Ok(n) => {
                        chunk.truncate(n);
                        Some(Ok(chunk))
                    }
                    Err(err) => Some(Err(ShellError::from(IoError::new(err, span, None)))),
                }
            })),
        span,
        signals,
        type_,
    );

    PipelineData::ByteStream(rebuilt, metadata)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_gather_env_vars() {
        let mut engine_state = EngineState::new();
        let symbols = r##" !"#$%&'()*+,-./:;<=>?@[\]^_`{|}~"##;

        gather_env_vars(
            [
                ("FOO".into(), "foo".into()),
                ("SYMBOLS".into(), symbols.into()),
                (symbols.into(), "symbols".into()),
            ]
            .into_iter(),
            &mut engine_state,
            Path::new("t"),
        );

        let env = engine_state.render_env_vars();

        assert!(matches!(env.get("FOO"), Some(&Value::String { val, .. }) if val == "foo"));
        assert!(matches!(env.get("SYMBOLS"), Some(&Value::String { val, .. }) if val == symbols));
        assert!(matches!(env.get(symbols), Some(&Value::String { val, .. }) if val == "symbols"));
        assert!(env.contains_key("PWD"));
        assert_eq!(env.len(), 4);
    }
}
