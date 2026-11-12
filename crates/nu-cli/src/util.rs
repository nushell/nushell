#![allow(clippy::byte_char_slices)]

use nu_cmd_base::hook::eval_hook;
use nu_engine::{eval_block, eval_block_with_early_return, exit::kill_jobs_and_exit};
use nu_parser::{Token, TokenContents, lex, parse, unescape_unquote_string};
use nu_protocol::{
    ByteStream, ByteStreamSource, ByteStreamType, ListStream, PipelineData, PipelineMetadata,
    Reader, ShellError, Signals, Span, Value, ValueIterator,
    ast::Block,
    block_is_bare_last_result_with,
    debugger::WithoutDebug,
    engine::{EngineState, LastResultSink, LastResultTruncation, Stack, StateWorkingSet},
    process::check_exit_status_future,
    report_error::report_compile_error,
    report_parse_error, report_parse_warning, report_shell_error,
    shell_error::generic::GenericError,
    truncate_list_item, truncate_value_to_budget, value_from_bytes, value_is_error_only,
};
#[cfg(windows)]
use nu_utils::enable_vt_processing;
use nu_utils::time::Instant;
use nu_utils::{escape_quote_string, perf};
use std::{io::Read, path::Path};

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
    let to_stderr = engine_state.is_mcp || engine_state.is_lsp || engine_state.is_dap;

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
        kill_jobs_and_exit(engine_state, *code)
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

/// Report every parse warning in `working_set`, each once. The parser can try a piece of code more
/// than once and leave the same warning behind each time, so a repeat at the same place is
/// skipped. [`report_parse_warning`] itself drops repeats of `--report first` warnings.
pub(crate) fn report_parse_warnings(stack: Option<&Stack>, working_set: &StateWorkingSet) {
    let mut reported: Vec<(Span, String)> = vec![];
    for warning in &working_set.parse_warnings {
        let key = (warning.span(), warning.to_string());
        if !reported.contains(&key) {
            report_parse_warning(stack, working_set, warning);
            reported.push(key);
        }
    }
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
        report_parse_warnings(Some(stack), &working_set);

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
    engine_state.merge_deletions(stack);

    // Set up interactive last-result before display_output transforms/prints the result. A value
    // is stored here. A stream is copied as display reads it and stored when it is dropped, after
    // display_output. Only for true REPL user lines (`capture_repl_last_result`), not config/env/banner.
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
/// Skips bare last-result retrievals. Values are stored right away. Streams are wrapped in a tee
/// that keeps them lazy, so they reach display as they are produced and `$ans.last` is stored
/// once they end.
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

    match pipeline_data {
        PipelineData::Empty => {
            stack.set_last_result(Value::nothing(Span::unknown()), None, budget);
            PipelineData::Empty
        }
        PipelineData::Value(value, metadata) => {
            if !value_is_error_only(&value) {
                stack.set_last_result(value.clone(), metadata.clone(), budget);
            }
            PipelineData::Value(value, metadata)
        }
        PipelineData::ListStream(stream, metadata) => tee_list_stream(
            stack,
            stream,
            metadata,
            budget,
            engine_state.signals().clone(),
        ),
        PipelineData::ByteStream(stream, metadata) => tee_byte_stream(
            stack,
            stream,
            metadata,
            budget,
            engine_state.signals().clone(),
        ),
    }
}

/// The part of a stream tee that stores its copy in `$ans.last`.
///
/// The copy is stored when the tee is dropped, however the stream ended: `$ans.last` is what passed
/// through, flagged as truncated if it went over budget or some was left unread (a display hook
/// that stops early, an error, Ctrl-C). If nothing was read at all, the previous `$ans.last` stays.
///
/// The previous `$ans.last` stays readable while the stream runs, so its lazy closures can still
/// use it, until it and the copy no longer fit in the budget together. It is freed then, so the
/// two never hold more than the budget between them.
struct StreamCapture {
    sink: LastResultSink,
    metadata: Option<PipelineMetadata>,
    signals: Signals,
    span: Span,
    budget: usize,
    /// [`Value::memory_size`] of the previous `$ans.last`, or `0` once it is freed.
    previous_size: usize,
    /// Part of the stream did not fit in the copy.
    overflowed: bool,
    /// The source ran out, so nothing was left unread.
    exhausted: bool,
}

impl StreamCapture {
    fn new(
        stack: &Stack,
        metadata: Option<PipelineMetadata>,
        signals: Signals,
        span: Span,
        budget: usize,
    ) -> Self {
        Self {
            sink: stack.last_result_sink(),
            metadata,
            signals,
            span,
            budget,
            previous_size: stack.last_result_memory_size(),
            overflowed: false,
            exhausted: false,
        }
    }

    /// The source has no more to give. An interrupted source says the same, but it wasn't done.
    fn source_ended(&mut self) {
        self.exhausted |= !self.signals.interrupted();
    }

    /// Call before the copy grows to `size`: frees the previous `$ans.last` if both wouldn't fit.
    fn make_room(&mut self, size: usize) {
        if self.previous_size > 0 && self.previous_size.saturating_add(size) > self.budget {
            self.sink.clear_payload();
            self.previous_size = 0;
        }
    }

    /// Whether a tee with an empty copy read nothing at all, rather than an empty stream.
    fn nothing_read(&self, copy_is_empty: bool) -> bool {
        copy_is_empty && !self.overflowed && !self.exhausted
    }

    /// Store `copy` as `$ans.last`, flagged with why it is less than the whole stream, if it is.
    ///
    /// Going over budget wins when both apply, since raising the limit is what the user can act on.
    fn store(&mut self, copy: Value) {
        let (stored, cut) = truncate_value_to_budget(copy, self.budget);
        if value_is_error_only(&stored) {
            return;
        }
        let truncation = if self.overflowed || cut {
            Some(LastResultTruncation::OverBudget)
        } else if self.exhausted {
            None
        } else {
            Some(LastResultTruncation::UnfinishedStream)
        };
        self.sink.store(stored, self.metadata.take(), truncation);
    }
}

/// Copies a list stream into `$ans.last` as its items pass through to the consumer.
///
/// Items are pulled only when the consumer asks for them, so capture never holds up display. See
/// [`StreamCapture`] for what gets stored and when.
struct ListStreamTee {
    iter: ValueIterator,
    capture: StreamCapture,
    kept: Vec<Value>,
    /// [`Value::memory_size`] of a list holding `kept`.
    used: usize,
}

impl ListStreamTee {
    /// Keep a copy of `item` if it fits. The first that doesn't ends the copy, the same way it ends
    /// a list cut down by [`truncate_value_to_budget`].
    fn copy_item(&mut self, item: &Value) {
        if self.capture.overflowed {
            return;
        }

        let budget = self.capture.budget;
        let used = self.used.saturating_add(item.memory_size());
        if used <= budget {
            self.capture.make_room(used);
            self.used = used;
            self.kept.push(item.clone());
        } else {
            self.capture.overflowed = true;
            self.capture.make_room(budget);
            let remaining = budget.saturating_sub(self.used);
            self.kept.extend(truncate_list_item(item, remaining));
        }
    }
}

impl Iterator for ListStreamTee {
    type Item = Value;

    fn next(&mut self) -> Option<Value> {
        let item = self.iter.next();
        match &item {
            Some(item) => self.copy_item(item),
            None => self.capture.source_ended(),
        }
        item
    }
}

impl Drop for ListStreamTee {
    fn drop(&mut self) {
        if !self.capture.nothing_read(self.kept.is_empty()) {
            let copy = Value::list(std::mem::take(&mut self.kept), self.capture.span);
            self.capture.store(copy);
        }
    }
}

/// Wrap a list stream in a [`ListStreamTee`].
fn tee_list_stream(
    stack: &Stack,
    stream: ListStream,
    metadata: Option<PipelineMetadata>,
    budget: usize,
    signals: Signals,
) -> PipelineData {
    let span = stream.span();
    let capture = StreamCapture::new(stack, metadata.clone(), signals, span, budget);
    let stream = stream.modify(|iter| ListStreamTee {
        iter,
        capture,
        kept: Vec::new(),
        used: Value::list(vec![], span).memory_size(),
    });
    PipelineData::ListStream(stream, metadata)
}

/// Copies a byte stream into `$ans.last` as its bytes pass through to the consumer.
///
/// See [`StreamCapture`] for what gets stored and when.
struct ByteStreamTee {
    reader: Reader,
    capture: StreamCapture,
    type_: ByteStreamType,
    trim_trailing_newline: bool,
    /// How much of the budget can hold bytes, once the value that wraps them is paid for.
    max_bytes: usize,
    prefix: Vec<u8>,
}

impl ByteStreamTee {
    fn new(
        reader: Reader,
        capture: StreamCapture,
        type_: ByteStreamType,
        trim_trailing_newline: bool,
    ) -> Self {
        Self {
            reader,
            type_,
            trim_trailing_newline,
            max_bytes: capture.budget.saturating_sub(std::mem::size_of::<Value>()),
            capture,
            prefix: Vec::new(),
        }
    }

    /// Append to `prefix`, growing it like [`Vec`] does but never past `max_bytes`.
    fn copy_bytes(&mut self, bytes: &[u8]) {
        let needed = self.prefix.len() + bytes.len();
        if needed > self.prefix.capacity() {
            let capacity = needed.max(self.prefix.capacity() * 2).min(self.max_bytes);
            self.capture
                .make_room(capacity.saturating_add(std::mem::size_of::<Value>()));
            self.prefix.reserve_exact(capacity - self.prefix.len());
        }
        self.prefix.extend_from_slice(bytes);
    }
}

impl Read for ByteStreamTee {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = self.reader.read(buf)?;
        if n == 0 && !buf.is_empty() {
            self.capture.source_ended();
        }

        let copied = n.min(self.max_bytes.saturating_sub(self.prefix.len()));
        self.capture.overflowed |= copied < n;
        self.copy_bytes(&buf[..copied]);
        Ok(n)
    }
}

impl Drop for ByteStreamTee {
    fn drop(&mut self) {
        if self.capture.nothing_read(self.prefix.is_empty()) {
            return;
        }

        // `Value::memory_size` counts capacity, so spare capacity would use up the budget.
        let mut prefix = std::mem::take(&mut self.prefix);
        prefix.shrink_to_fit();

        // Decode for `$ans.last` the same way collecting a byte stream does:
        // Binary stays binary; String/Unknown become string when UTF-8, else binary.
        let whole = self.capture.exhausted && !self.capture.overflowed;
        let copy = value_from_bytes(
            prefix,
            self.capture.span,
            self.type_,
            self.trim_trailing_newline && whole,
        );
        self.capture.store(copy);
    }
}

/// Wrap a byte stream in a [`ByteStreamTee`], unless it has nothing to capture.
fn tee_byte_stream(
    stack: &Stack,
    stream: ByteStream,
    metadata: Option<PipelineMetadata>,
    budget: usize,
    signals: Signals,
) -> PipelineData {
    let span = stream.span();
    let type_ = stream.type_();
    let stream_signals = stream.signals().clone();
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

    let Some(reader) = stream.reader() else {
        // Defensive: source said stdout exists but reader failed. Prior `$ans` left intact;
        // nothing left to print.
        return PipelineData::Empty;
    };

    let capture = StreamCapture::new(stack, metadata.clone(), signals, span, budget);
    let tee = ByteStreamTee::new(reader, capture, type_, trim_trailing_newline);
    PipelineData::ByteStream(ByteStream::read(tee, span, stream_signals, type_), metadata)
}

#[cfg(test)]
mod test {
    use super::*;
    use nu_protocol::LAST_VARIABLE_ID;
    use std::sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    };

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

    /// `unlet` on a constant in a nested block hides it only in that block, so the next source
    /// unit (like the next REPL line) still has it, though the block ran on the same stack.
    #[test]
    fn unlet_constant_in_nested_block_keeps_it_for_later_source() -> Result<(), ShellError> {
        let mut engine_state =
            nu_command::add_shell_command_context(nu_cmd_lang::create_default_context());
        let mut stack = Stack::new();
        for source in ["const x = 1", "if true { unlet $x }"] {
            let failed = evaluate_source(
                &mut engine_state,
                &mut stack,
                source.as_bytes(),
                "test",
                PipelineData::empty(),
                false,
            )?;
            assert!(!failed, "`{source}` failed");
        }

        assert!(
            StateWorkingSet::new(&engine_state)
                .find_variable(b"$x")
                .is_some()
        );
        Ok(())
    }

    const ROOMY_BUDGET: usize = 1024 * 1024;

    /// `$ans.last` as currently stored, if any.
    fn stored_last(stack: &Stack) -> Option<Value> {
        stack
            .get_var(LAST_VARIABLE_ID, Span::test_data())
            .ok()?
            .get_data_by_key("last")
    }

    fn test_ints(range: std::ops::Range<i64>) -> Value {
        Value::test_list(range.map(Value::test_int).collect())
    }

    /// A list stream of `0..len`, tee'd, and a count of the items pulled from its source.
    fn teed_list_stream(stack: &Stack, len: i64, budget: usize) -> (ListStream, Arc<AtomicUsize>) {
        let pulled = Arc::new(AtomicUsize::new(0));
        let counter = pulled.clone();
        let source = (0..len).map(move |i| {
            counter.fetch_add(1, Ordering::Relaxed);
            Value::test_int(i)
        });
        let source = ListStream::new(source, Span::test_data(), Signals::empty());
        match tee_list_stream(stack, source, None, budget, Signals::empty()) {
            PipelineData::ListStream(stream, _) => (stream, pulled),
            other => panic!("expected a list stream, got {other:?}"),
        }
    }

    /// A byte stream of `chunks`, tee'd, and a count of the chunks pulled from its source.
    fn teed_byte_stream(
        stack: &Stack,
        chunks: &'static [&'static str],
        budget: usize,
    ) -> (Reader, Arc<AtomicUsize>) {
        let pulled = Arc::new(AtomicUsize::new(0));
        let counter = pulled.clone();
        let source = chunks.iter().map(move |chunk| {
            counter.fetch_add(1, Ordering::Relaxed);
            *chunk
        });
        let source = ByteStream::from_iter(
            source,
            Span::test_data(),
            Signals::empty(),
            ByteStreamType::String,
        );
        match tee_byte_stream(stack, source, None, budget, Signals::empty()) {
            PipelineData::ByteStream(stream, _) => {
                (stream.reader().expect("a readable stream"), pulled)
            }
            other => panic!("expected a byte stream, got {other:?}"),
        }
    }

    /// One `read` from `reader`, as a string.
    fn read_chunk(reader: &mut Reader) -> String {
        let mut buf = [0; 64];
        let n = reader.read(&mut buf).expect("read should succeed");
        String::from_utf8_lossy(&buf[..n]).into_owned()
    }

    #[test]
    fn list_tee_pulls_only_what_the_consumer_asks_for() {
        let stack = Stack::new();
        let (mut stream, pulled) = teed_list_stream(&stack, 3, ROOMY_BUDGET);
        assert_eq!(pulled.load(Ordering::Relaxed), 0);

        assert_eq!(stream.next_value(), Some(Value::test_int(0)));
        assert_eq!(pulled.load(Ordering::Relaxed), 1);
        assert_eq!(
            stored_last(&stack),
            None,
            "stored only once the stream ends"
        );

        while stream.next_value().is_some() {}
        drop(stream);
        assert_eq!(stored_last(&stack), Some(test_ints(0..3)));
        assert!(!stack.last_result_was_truncated());
    }

    #[test]
    fn list_tee_over_budget_still_passes_everything_through() {
        let stack = Stack::new();
        let budget = test_ints(0..2).memory_size();
        let (stream, _) = teed_list_stream(&stack, 5, budget);

        assert_eq!(stream.into_iter().count(), 5);
        assert_eq!(stored_last(&stack), Some(test_ints(0..2)));
        assert!(stack.last_result_was_truncated());
    }

    #[test]
    fn list_tee_stopped_early_stores_what_passed_through() {
        let stack = Stack::new();
        let (mut stream, pulled) = teed_list_stream(&stack, 5, ROOMY_BUDGET);

        stream.next_value();
        stream.next_value();
        drop(stream);
        assert_eq!(pulled.load(Ordering::Relaxed), 2);
        assert_eq!(stored_last(&stack), Some(test_ints(0..2)));
        assert!(stack.last_result_was_truncated());
    }

    #[test]
    fn list_tee_dropped_unread_keeps_the_previous_result() {
        let mut stack = Stack::new();
        stack.set_last_result(Value::test_int(42), None, ROOMY_BUDGET);

        let (stream, pulled) = teed_list_stream(&stack, 5, ROOMY_BUDGET);
        drop(stream);
        assert_eq!(pulled.load(Ordering::Relaxed), 0);
        assert_eq!(stored_last(&stack), Some(Value::test_int(42)));
        assert!(!stack.last_result_was_truncated());
    }

    /// An interrupted source ends like one that ran out, but there may have been more of it.
    #[test]
    fn list_tee_interrupted_stores_what_passed_through_as_truncated() {
        let stack = Stack::new();
        let signals = Signals::new(Arc::new(AtomicBool::new(false)));
        let source = ListStream::new(
            (0..5).map(Value::test_int),
            Span::test_data(),
            signals.clone(),
        );
        let PipelineData::ListStream(mut stream, _) =
            tee_list_stream(&stack, source, None, ROOMY_BUDGET, signals.clone())
        else {
            panic!("expected a list stream");
        };

        stream.next_value();
        stream.next_value();
        signals.trigger();
        assert_eq!(stream.next_value(), None);
        drop(stream);
        assert_eq!(stored_last(&stack), Some(test_ints(0..2)));
        assert!(stack.last_result_was_truncated());
    }

    #[test]
    fn list_tee_frees_the_previous_result_once_both_do_not_fit() {
        let mut stack = Stack::new();
        // A list costs as much as an int, so this is room for 5 ints: the previous result takes 3.
        let budget = test_ints(0..4).memory_size();
        stack.set_last_result(test_ints(10..12), None, budget);

        let (mut stream, _) = teed_list_stream(&stack, 4, budget);
        stream.next_value();
        assert_eq!(
            stored_last(&stack),
            Some(test_ints(10..12)),
            "both still fit"
        );

        stream.next_value();
        assert_eq!(stored_last(&stack), None, "both no longer fit");

        while stream.next_value().is_some() {}
        drop(stream);
        assert_eq!(stored_last(&stack), Some(test_ints(0..4)));
        assert!(!stack.last_result_was_truncated());
    }

    #[test]
    fn byte_tee_reads_only_what_the_consumer_asks_for() {
        let stack = Stack::new();
        let (mut reader, pulled) = teed_byte_stream(&stack, &["ab", "cd"], ROOMY_BUDGET);
        assert_eq!(pulled.load(Ordering::Relaxed), 0);

        assert_eq!(read_chunk(&mut reader), "ab");
        assert_eq!(pulled.load(Ordering::Relaxed), 1);
        assert_eq!(
            stored_last(&stack),
            None,
            "stored only once the stream ends"
        );

        assert_eq!(read_chunk(&mut reader), "cd");
        assert_eq!(read_chunk(&mut reader), "");
        drop(reader);
        assert_eq!(stored_last(&stack), Some(Value::test_string("abcd")));
        assert!(!stack.last_result_was_truncated());
    }

    #[test]
    fn byte_tee_over_budget_still_passes_everything_through() {
        let stack = Stack::new();
        let budget = std::mem::size_of::<Value>() + 3;
        let (mut reader, _) = teed_byte_stream(&stack, &["ab", "cd", "ef"], budget);

        let mut seen = String::new();
        reader
            .read_to_string(&mut seen)
            .expect("read should succeed");
        assert_eq!(seen, "abcdef");
        drop(reader);
        assert_eq!(stored_last(&stack), Some(Value::test_string("abc")));
        assert!(stack.last_result_was_truncated());
    }

    #[test]
    fn byte_tee_stopped_early_stores_what_passed_through() {
        let stack = Stack::new();
        let (mut reader, pulled) = teed_byte_stream(&stack, &["ab", "cd"], ROOMY_BUDGET);

        assert_eq!(read_chunk(&mut reader), "ab");
        drop(reader);
        assert_eq!(pulled.load(Ordering::Relaxed), 1);
        assert_eq!(stored_last(&stack), Some(Value::test_string("ab")));
        assert!(stack.last_result_was_truncated());
    }

    #[test]
    fn byte_tee_dropped_unread_keeps_the_previous_result() {
        let mut stack = Stack::new();
        stack.set_last_result(Value::test_int(42), None, ROOMY_BUDGET);

        let (reader, pulled) = teed_byte_stream(&stack, &["ab"], ROOMY_BUDGET);
        drop(reader);
        assert_eq!(pulled.load(Ordering::Relaxed), 0);
        assert_eq!(stored_last(&stack), Some(Value::test_int(42)));
        assert!(!stack.last_result_was_truncated());
    }

    /// `Value::memory_size` counts capacity, so what's left over from growing mustn't be kept.
    #[test]
    fn byte_tee_copy_that_fits_is_not_flagged_for_its_spare_capacity() {
        let stack = Stack::new();
        let budget = std::mem::size_of::<Value>() + 7;
        let (mut reader, _) = teed_byte_stream(&stack, &["ab", "cd", "ef"], budget);

        let mut seen = String::new();
        reader
            .read_to_string(&mut seen)
            .expect("read should succeed");
        drop(reader);
        assert_eq!(stored_last(&stack), Some(Value::test_string("abcdef")));
        assert!(!stack.last_result_was_truncated());
        assert_eq!(
            stack.last_result_memory_size(),
            std::mem::size_of::<Value>() + 6
        );
    }

    /// The reader checks for an interrupt before it reads, so one that lands during the read looks
    /// like the end of the stream.
    #[test]
    fn byte_tee_interrupted_during_a_read_stores_what_passed_through_as_truncated() {
        let stack = Stack::new();
        let signals = Signals::new(Arc::new(AtomicBool::new(false)));
        let interrupt = signals.clone();
        let source = std::iter::once("ab").chain(std::iter::from_fn(move || {
            interrupt.trigger();
            None
        }));
        let source = ByteStream::from_iter(
            source,
            Span::test_data(),
            Signals::empty(),
            ByteStreamType::String,
        );
        let PipelineData::ByteStream(stream, _) =
            tee_byte_stream(&stack, source, None, ROOMY_BUDGET, signals)
        else {
            panic!("expected a byte stream");
        };
        let mut reader = stream.reader().expect("a readable stream");

        assert_eq!(read_chunk(&mut reader), "ab");
        assert_eq!(read_chunk(&mut reader), "");
        drop(reader);
        assert_eq!(stored_last(&stack), Some(Value::test_string("ab")));
        assert!(stack.last_result_was_truncated());
    }

    #[test]
    fn byte_tee_frees_the_previous_result_once_both_do_not_fit() {
        let mut stack = Stack::new();
        let previous = Value::test_string("x".repeat(50));
        let budget = previous.memory_size() + std::mem::size_of::<Value>() + 50;
        stack.set_last_result(previous.clone(), None, budget);

        let chunks = &["0123456789012345678901234567890123456789012345678 ", "tail"];
        let (mut reader, _) = teed_byte_stream(&stack, chunks, budget);
        read_chunk(&mut reader);
        assert_eq!(stored_last(&stack), Some(previous), "both still fit");

        read_chunk(&mut reader);
        assert_eq!(stored_last(&stack), None, "both no longer fit");

        read_chunk(&mut reader);
        drop(reader);
        assert_eq!(
            stored_last(&stack),
            Some(Value::test_string(chunks.concat()))
        );
        assert!(!stack.last_result_was_truncated());
    }

    #[test]
    fn byte_tee_copy_never_grows_past_the_budget() {
        let stack = Stack::new();
        let source = ByteStream::from_iter(
            ["x".repeat(60), "x".repeat(20), "x".repeat(60)],
            Span::test_data(),
            Signals::empty(),
            ByteStreamType::String,
        );
        let reader = source.reader().expect("a readable stream");
        let budget = std::mem::size_of::<Value>() + 100;
        let capture = StreamCapture::new(&stack, None, Signals::empty(), Span::test_data(), budget);
        let mut tee = ByteStreamTee::new(reader, capture, ByteStreamType::String, false);

        let mut buf = [0; 64];
        while tee.read(&mut buf).expect("read should succeed") > 0 {}
        assert_eq!(tee.prefix.len(), 100);
        assert_eq!(tee.prefix.capacity(), 100);
    }
}
