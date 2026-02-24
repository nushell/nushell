use crate::history::History;
use miette::{Diagnostic, SourceCode, SourceSpan};
use nu_protocol::{
    PipelineData, PipelineExecutionData, Signals, Span, Value,
    debugger::WithoutDebug,
    engine::{EngineState, Stack, StateWorkingSet},
};
use std::{
    sync::{Arc, atomic::AtomicBool},
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

const OUTPUT_LIMIT_ENV_VAR: &str = "NU_MCP_OUTPUT_LIMIT";
const DEFAULT_OUTPUT_LIMIT: usize = 10 * 1024; // 10kb

/// Formats a miette Diagnostic error as a NUON record for MCP.
///
/// Extracts structured error information (code, message, help, labels with spans)
/// and formats it as NUON - a machine-readable format that's more useful for LLMs
/// than the human-readable display format.
///
/// The output includes:
/// - `code`: Error code (e.g., "nu::parser::parse_mismatch")
/// - `msg`: Short error message
/// - `severity`: "error", "warning", or "advice" (if available)
/// - `help`: Hint/suggestion for fixing the error (if available)
/// - `url`: Documentation URL (if available)
/// - `labels`: List of source locations with context:
///   - `text`: What the label is pointing at (e.g., "expected duration")
///   - `span`: The exact text that caused the error
///   - `line`: 1-indexed line number
///   - `column`: 1-indexed column number
fn format_mcp_error(
    working_set: &StateWorkingSet,
    error: &dyn Diagnostic,
    default_code: Option<&'static str>,
) -> String {
    let mut record = nu_protocol::record! {};

    // Error code (e.g., "nu::parser::parse_mismatch")
    let code = error
        .code()
        .map(|c| c.to_string())
        .or_else(|| default_code.map(String::from));
    if let Some(code) = code {
        record.push("code", Value::string(code, Span::unknown()));
    }

    // Error message from Display trait
    record.push("msg", Value::string(error.to_string(), Span::unknown()));

    // Severity level (error, warning, advice)
    if let Some(severity) = error.severity() {
        let severity_str = match severity {
            miette::Severity::Error => "error",
            miette::Severity::Warning => "warning",
            miette::Severity::Advice => "advice",
        };
        record.push("severity", Value::string(severity_str, Span::unknown()));
    }

    // Help/hint text if available
    if let Some(help) = error.help() {
        record.push("help", Value::string(help.to_string(), Span::unknown()));
    }

    // Documentation URL if available
    if let Some(url) = error.url() {
        record.push("url", Value::string(url.to_string(), Span::unknown()));
    }

    // Labels with span information, line/column, and source context
    if let Some(labels) = error.labels() {
        let labels_list: Vec<Value> = labels
            .map(|label| {
                let mut label_record = nu_protocol::record! {};

                // Label text/message (what it's pointing at, e.g., "expected duration")
                if let Some(text) = label.label() {
                    label_record.push("text", Value::string(text, Span::unknown()));
                }

                // Extract source context with line/column info
                let span: SourceSpan = *label.inner();
                if let Some((span_text, line, column)) = extract_source_context(working_set, &span)
                {
                    // The exact source text at the error span
                    label_record.push("span", Value::string(span_text, Span::unknown()));
                    // 1-indexed line and column for human readability
                    label_record.push("line", Value::int(line as i64, Span::unknown()));
                    label_record.push("column", Value::int(column as i64, Span::unknown()));
                }

                Value::record(label_record, Span::unknown())
            })
            .collect();

        if !labels_list.is_empty() {
            record.push("labels", Value::list(labels_list, Span::unknown()));
        }
    }

    // Convert to NUON format
    let value = Value::record(record, Span::unknown());
    nuon::to_nuon(
        working_set.permanent(),
        &value,
        nuon::ToNuonConfig::default()
            .style(nuon::ToStyle::Raw)
            .span(Some(Span::unknown())),
    )
    .unwrap_or_else(|_| error.to_string())
}

/// Extract the source code context around a span for error display.
/// Returns (span_text, line_number, column_number) where line/column are 1-indexed.
fn extract_source_context(
    working_set: &StateWorkingSet,
    span: &SourceSpan,
) -> Option<(String, usize, usize)> {
    // Use the working_set as the source code provider (it implements miette::SourceCode)
    let contents = working_set.read_span(span, 0, 0).ok()?;

    // Get the source text from the span data (it's &[u8])
    let source = contents.data();
    let span_text = if source.is_empty() {
        String::new()
    } else {
        String::from_utf8_lossy(source).into_owned()
    };

    // SpanContents provides 0-indexed line/column, convert to 1-indexed for humans
    let line = contents.line() + 1;
    let column = contents.column() + 1;

    Some((span_text, line, column))
}

/// Creates an invalid_params MCP error for user input errors (parse/compile errors).
///
/// Uses error code -32602 (Invalid params) since these are user input errors, not server errors.
/// Error is formatted as NUON for machine-readable structured output.
fn user_input_error(
    working_set: &StateWorkingSet,
    error: &dyn Diagnostic,
    default_code: Option<&'static str>,
) -> rmcp::ErrorData {
    rmcp::ErrorData::invalid_params(format_mcp_error(working_set, error, default_code), None)
}

/// Creates an internal MCP error for runtime errors.
///
/// Uses error code -32603 (Internal error) since these are server-side execution errors.
/// Error is formatted as NUON for machine-readable structured output.
pub(crate) fn shell_error_to_mcp_error(
    error: nu_protocol::ShellError,
    engine_state: &EngineState,
) -> rmcp::ErrorData {
    let working_set = StateWorkingSet::new(engine_state);
    rmcp::ErrorData::internal_error(
        format_mcp_error(&working_set, &error, Some("nu::shell::error")),
        None,
    )
}

/// MCP error for cancelled operations.
fn cancelled_error() -> rmcp::ErrorData {
    rmcp::ErrorData::internal_error("Operation cancelled by client".to_string(), None)
}

/// Evaluates Nushell code in a persistent REPL-style context for MCP.
///
/// # Architecture
///
/// The evaluator maintains a persistent `EngineState` and `Stack` that carry
/// state across evaluations—just like an interactive REPL session. Each evaluation:
/// 1. Parses code into a `Block` and gets a `StateDelta` via `working_set.render()`
/// 2. **Merges the delta** into the persistent engine state
/// 3. Evaluates the block with the persistent state and stack
///
/// Step 2 ensures parsed blocks (including closures) are registered and available.
///
/// # Cancellation Support
///
/// The evaluator supports cancellation via `CancellationToken`. When cancelled:
/// 1. The evaluation is interrupted via nushell's `Signals` mechanism
/// 2. Any forked state changes are discarded
/// 3. The original state remains unchanged
///
/// # State Persistence
///
/// Variables, definitions, and environment changes persist across calls,
/// enabling workflows like:
/// ```nu
/// $env.MY_VAR = "hello"  # First call
/// $env.MY_VAR            # Second call returns "hello"
/// ```
///
/// # History
///
/// The evaluator maintains a `$history` list that stores all command outputs.
/// Each evaluation can access previous outputs via `$history.0`, `$history.1`, etc.
/// History is stored as a ring buffer with a configurable limit (default: 100 entries) via
/// `NU_MCP_HISTORY_LIMIT` env var. When the limit is reached, oldest entries are evicted.
/// Large outputs are truncated in the response but stored in full in history.
pub struct Evaluator {
    state: Mutex<EvalState>,
}

/// The mutable evaluation state that persists across evaluations.
struct EvalState {
    engine_state: EngineState,
    stack: Stack,
    history: History,
}

impl EvalState {
    /// Creates a forked copy of the state for isolated evaluation.
    ///
    /// The forked state has its own `Signals` instance that can be triggered
    /// to interrupt the evaluation without affecting the original state.
    ///
    /// Returns `(forked_state, interrupt_trigger)` where `interrupt_trigger`
    /// is an `Arc<AtomicBool>` that can be set to `true` to interrupt the evaluation.
    fn fork(&self) -> (Self, Arc<AtomicBool>) {
        let interrupt = Arc::new(AtomicBool::new(false));
        let signals = Signals::new(interrupt.clone());

        let mut engine_state = self.engine_state.clone();
        engine_state.set_signals(signals);

        // Create a child stack that inherits from current state
        // We clone instead of using parent linking since we may discard entirely
        let stack = self.stack.clone();

        // Clone history so changes can be discarded
        let history = self.history.clone();

        (
            Self {
                engine_state,
                stack,
                history,
            },
            interrupt,
        )
    }
}

impl Evaluator {
    pub fn new(mut engine_state: EngineState) -> Self {
        // Disable ANSI coloring for MCP - it's a computer-to-computer protocol
        let mut config = nu_protocol::Config::clone(engine_state.get_config());
        config.use_ansi_coloring = nu_protocol::UseAnsiColoring::False;
        engine_state.set_config(config);

        let history = History::new(&mut engine_state);

        Self {
            state: Mutex::new(EvalState {
                engine_state,
                // Use capture_all() to capture external command stdout AND stderr
                // instead of letting them go to the process's terminal (OutDest::Inherit)
                stack: Stack::new().capture_all(),
                history,
            }),
        }
    }

    /// Evaluates nushell source code with cancellation support.
    ///
    /// This method:
    /// 1. Forks the current state (cheap due to Arc-based copy-on-write)
    /// 2. Runs the evaluation on the forked state in a blocking task
    /// 3. Races the evaluation against the cancellation token
    /// 4. On success: merges changes back to the main state
    /// 5. On cancellation: discards the forked state, original unchanged
    pub async fn eval_async(
        &self,
        nu_source: &str,
        ct: CancellationToken,
    ) -> Result<String, rmcp::ErrorData> {
        // Fork the state for isolated evaluation
        let (forked_state, interrupt) = {
            let state = self.state.lock().await;
            state.fork()
        };

        let source = nu_source.to_string();

        // Run evaluation in a blocking task since eval_block is synchronous
        let eval_handle = tokio::task::spawn_blocking(move || eval_inner(forked_state, &source));

        // Set up cancellation monitoring
        let abort_handle = eval_handle.abort_handle();

        // Spawn a task to trigger interrupt on cancellation
        let interrupt_clone = interrupt.clone();
        let ct_clone = ct.clone();
        tokio::spawn(async move {
            ct_clone.cancelled().await;
            // Trigger nushell's interrupt signal to stop any running commands
            interrupt_clone.store(true, std::sync::atomic::Ordering::Relaxed);
            // Abort the blocking task
            abort_handle.abort();
        });

        // Wait for evaluation to complete
        match eval_handle.await {
            Ok((new_state, eval_result)) => {
                // Check if we were cancelled
                if ct.is_cancelled() {
                    return Err(cancelled_error());
                }
                // Commit the forked state back to main state
                let mut state = self.state.lock().await;
                *state = new_state;
                eval_result
            }
            Err(join_error) => {
                if join_error.is_cancelled() {
                    Err(cancelled_error())
                } else {
                    Err(rmcp::ErrorData::internal_error(
                        format!("Evaluation task panicked: {join_error}"),
                        None,
                    ))
                }
            }
        }
    }

    /// Synchronous evaluation without cancellation support.
    ///
    /// Provided for backwards compatibility and testing.
    #[cfg(test)]
    pub fn eval(&self, nu_source: &str) -> Result<String, rmcp::ErrorData> {
        // Create a runtime for sync evaluation in tests
        let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
        rt.block_on(self.eval_async(nu_source, CancellationToken::new()))
    }
}

/// Inner evaluation logic that operates on an owned `EvalState`.
///
/// Returns the (possibly modified) state along with the result.
/// This allows the caller to decide whether to commit or discard the state.
fn eval_inner(
    mut state: EvalState,
    nu_source: &str,
) -> (EvalState, Result<String, rmcp::ErrorData>) {
    let EvalState {
        engine_state,
        stack,
        history,
    } = &mut state;

    let result = eval_on_state(engine_state, stack, history, nu_source);
    (state, result)
}

/// Core evaluation logic shared by both sync and async paths.
fn eval_on_state(
    engine_state: &mut EngineState,
    stack: &mut Stack,
    history: &mut History,
    nu_source: &str,
) -> Result<String, rmcp::ErrorData> {
    let (block, delta) = {
        let mut working_set = StateWorkingSet::new(engine_state);
        let block = nu_parser::parse(&mut working_set, None, nu_source.as_bytes(), false);

        if let Some(err) = working_set.parse_errors.first() {
            return Err(user_input_error(&working_set, err, None));
        }

        if let Some(err) = working_set.compile_errors.first() {
            return Err(user_input_error(&working_set, err, None));
        }

        (block, working_set.render())
    };

    engine_state
        .merge_delta(delta)
        .map_err(|e| shell_error_to_mcp_error(e, engine_state))?;

    // Set up $history variable on the stack before evaluation
    stack.add_var(history.var_id(), history.as_value());

    let output =
        nu_engine::eval_block::<WithoutDebug>(engine_state, stack, &block, PipelineData::empty())
            .map_err(|e| shell_error_to_mcp_error(e, engine_state))?;

    let cwd = engine_state
        .cwd(Some(stack))
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| String::from("unknown"));

    let (output_value, output_nuon) = process_pipeline(output, engine_state)?;

    // Create timestamp for response
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as i64)
        .unwrap_or(0);
    let timestamp_value = chrono::DateTime::from_timestamp_nanos(timestamp).fixed_offset();

    // Store in history
    let history_index = history.push(output_value, engine_state, stack);

    let truncated =
        output_limit(engine_state, stack).is_some_and(|limit| output_nuon.len() > limit);

    let mut record = nu_protocol::record! {
        "cwd" => Value::string(cwd, Span::unknown()),
        "history_index" => Value::int(history_index as i64, Span::unknown()),
        "timestamp" => Value::date(timestamp_value, Span::unknown()),
    };

    if truncated {
        record.push(
            "note",
            Value::string(
                format!("output truncated, full result in $history.{history_index}"),
                Span::unknown(),
            ),
        );
    } else {
        record.push("output", Value::string(output_nuon, Span::unknown()));
    }

    let response = Value::record(record, Span::unknown());

    nuon::to_nuon(
        engine_state,
        &response,
        nuon::ToNuonConfig::default()
            .style(nuon::ToStyle::Raw)
            .span(Some(Span::unknown())),
    )
    .map_err(|e| shell_error_to_mcp_error(e, engine_state))
}

/// Returns the output limit in bytes.
///
/// Defaults to 10kb. Can be overridden via `NU_MCP_OUTPUT_LIMIT` env var.
/// Set to `0` to disable truncation entirely.
fn output_limit(engine_state: &EngineState, stack: &Stack) -> Option<usize> {
    let limit = stack
        .get_env_var(engine_state, OUTPUT_LIMIT_ENV_VAR)
        .and_then(|v| v.as_filesize().ok())
        .and_then(|fs| usize::try_from(fs.get()).ok())
        .unwrap_or(DEFAULT_OUTPUT_LIMIT);

    if limit == 0 { None } else { Some(limit) }
}

fn process_pipeline(
    pipeline_execution_data: PipelineExecutionData,
    engine_state: &EngineState,
) -> Result<(Value, String), rmcp::ErrorData> {
    let span = pipeline_execution_data.span();

    if let PipelineData::ByteStream(stream, ..) = pipeline_execution_data.body {
        // Try to handle as a child process first (external commands)
        // This properly handles both stdout and stderr when capture_all() is used
        match stream.into_child() {
            Ok(child) => {
                let output = child
                    .wait_with_output()
                    .map_err(|e| shell_error_to_mcp_error(e, engine_state))?;

                // Combine stdout and stderr into a single output
                let mut combined = Vec::new();
                if let Some(stdout) = output.stdout {
                    combined.extend(stdout);
                }
                if let Some(stderr) = output.stderr {
                    if !combined.is_empty() && !stderr.is_empty() {
                        combined.push(b'\n');
                    }
                    combined.extend(stderr);
                }

                let string_output = String::from_utf8_lossy(&combined).into_owned();
                let value = Value::string(&string_output, Span::unknown());
                return Ok((value, string_output));
            }
            Err(stream) => {
                // Not a child process (e.g., Read or File source), use write_to
                let mut buffer = Vec::new();
                stream
                    .write_to(&mut buffer)
                    .map_err(|e| shell_error_to_mcp_error(e, engine_state))?;
                let string_output = String::from_utf8_lossy(&buffer).into_owned();
                let value = Value::string(&string_output, Span::unknown());
                return Ok((value, string_output));
            }
        }
    }

    let mut values = Vec::new();
    for item in pipeline_execution_data.body {
        if let Value::Error { error, .. } = &item {
            return Err(shell_error_to_mcp_error(*error.clone(), engine_state));
        }
        values.push(item);
    }

    let value_to_store = match values.len() {
        1 => values
            .pop()
            .expect("values has exactly one element; this cannot fail"),
        _ => Value::list(values, span.unwrap_or(Span::unknown())),
    };

    let nuon_string = nuon::to_nuon(
        engine_state,
        &value_to_store,
        nuon::ToNuonConfig::default()
            .style(nuon::ToStyle::Raw)
            .span(Some(Span::unknown())),
    )
    .map_err(|e| shell_error_to_mcp_error(e, engine_state))?;

    Ok((value_to_store, nuon_string))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evaluator_response_format() -> Result<(), Box<dyn std::error::Error>> {
        let engine_state = nu_cmd_lang::create_default_context();
        let evaluator = Evaluator::new(engine_state);
        let result = evaluator.eval("42")?;

        assert!(
            result.contains("history_index"),
            "Response should contain history_index, got: {result}"
        );
        assert!(
            result.contains("cwd"),
            "Response should contain cwd, got: {result}"
        );
        assert!(
            result.contains("timestamp"),
            "Response should contain timestamp, got: {result}"
        );
        assert!(
            result.contains("output"),
            "Response should contain output, got: {result}"
        );
        assert!(
            result.contains("42"),
            "Output should contain the evaluated value, got: {result}"
        );
        Ok(())
    }

    #[test]
    fn test_history_index_increments() -> Result<(), Box<dyn std::error::Error>> {
        let engine_state = nu_cmd_lang::create_default_context();
        let evaluator = Evaluator::new(engine_state);

        let result1 = evaluator.eval("1")?;
        let result2 = evaluator.eval("2")?;
        let result3 = evaluator.eval("3")?;

        assert!(
            result1.contains("history_index:0") || result1.contains("history_index: 0"),
            "First result should have history_index: 0, got: {result1}"
        );
        assert!(
            result2.contains("history_index:1") || result2.contains("history_index: 1"),
            "Second result should have history_index: 1, got: {result2}"
        );
        assert!(
            result3.contains("history_index:2") || result3.contains("history_index: 2"),
            "Third result should have history_index: 2, got: {result3}"
        );
        Ok(())
    }

    #[test]
    fn test_history_variable_exists() -> Result<(), Box<dyn std::error::Error>> {
        let engine_state = nu_cmd_lang::create_default_context();
        let evaluator = Evaluator::new(engine_state);

        evaluator.eval("42")?;

        let result = evaluator.eval("$history")?;

        assert!(
            result.contains("output"),
            "Should be able to access $history, got: {result}"
        );
        assert!(
            result.contains("42"),
            "History should contain previous output 42, got: {result}"
        );
        Ok(())
    }

    #[test]
    fn test_history_access_by_index() -> Result<(), Box<dyn std::error::Error>> {
        let engine_state = nu_cmd_lang::create_default_context();
        let evaluator = Evaluator::new(engine_state);

        evaluator.eval("100")?;
        evaluator.eval("200")?;

        let result = evaluator.eval("$history.0")?;
        assert!(
            result.contains("100"),
            "history.0 should be 100, got: {result}"
        );

        let result = evaluator.eval("$history.1")?;
        assert!(
            result.contains("200"),
            "history.1 should be 200, got: {result}"
        );

        Ok(())
    }

    #[test]
    fn test_output_truncation() -> Result<(), Box<dyn std::error::Error>> {
        let engine_state = nu_cmd_lang::create_default_context();
        let evaluator = Evaluator::new(engine_state);

        evaluator.eval("$env.NU_MCP_OUTPUT_LIMIT = 20b")?;

        let result =
            evaluator.eval("\"this is a very long string that exceeds the output limit\"")?;

        // Should have 'note' field instead of 'output' when truncated
        assert!(
            result.contains("note") && result.contains("truncated") && result.contains("$history"),
            "Large output should have note about truncation, got: {result}"
        );
        assert!(
            !result.contains("output:"),
            "Truncated response should not have output field, got: {result}"
        );

        Ok(())
    }

    #[test]
    fn test_evaluator_parse_error_nuon_format() {
        let engine_state = nu_cmd_lang::create_default_context();
        let evaluator = Evaluator::new(engine_state);

        let result = evaluator.eval("let x = [1, 2, 3");

        assert!(result.is_err());
        let err = result.unwrap_err();
        let err_msg = err.message.to_string();

        // Error should be in NUON format with structured fields
        assert!(
            err_msg.contains("code:") && err_msg.contains("nu::parser::unexpected_eof"),
            "Error message should contain code field with 'nu::parser::unexpected_eof', but got: {err_msg}"
        );

        assert!(
            err_msg.contains("msg:"),
            "Error message should contain msg field, but got: {err_msg}"
        );

        assert!(
            err_msg.contains("labels:"),
            "Error message should contain labels field, but got: {err_msg}"
        );

        // Labels should include line and column numbers (in NUON table format)
        // Format is: labels:[[text,span,line,column];[...values...]]
        assert!(
            err_msg.contains(",line,") || err_msg.contains("line:"),
            "Error labels should contain line number, but got: {err_msg}"
        );

        assert!(
            err_msg.contains(",column]") || err_msg.contains("column:"),
            "Error labels should contain column number, but got: {err_msg}"
        );

        assert!(
            !err_msg.contains('\x1b'),
            "Error message should not contain ANSI escape codes, but got: {err_msg:?}"
        );

        assert!(
            !err_msg.contains("Span {"),
            "Error message should not contain raw Debug formatting, but got: {err_msg}"
        );
    }

    #[test]
    fn test_evaluator_compile_error_nuon_format() {
        let engine_state = nu_cmd_lang::create_default_context();
        let evaluator = Evaluator::new(engine_state);

        let result = evaluator.eval("[{a: 1}] | get a");

        assert!(result.is_err());
        let err = result.unwrap_err();
        let err_msg = err.message.to_string();

        // Error should be in NUON format with structured fields
        assert!(
            err_msg.contains("code:") && err_msg.contains("nu::compile::"),
            "Error message should contain code field with 'nu::compile::', but got: {err_msg}"
        );

        assert!(
            err_msg.contains("msg:"),
            "Error message should contain msg field, but got: {err_msg}"
        );

        assert!(
            !err_msg.contains("Span {"),
            "Error message should not contain raw Debug formatting, but got: {err_msg}"
        );
    }

    #[test]
    fn test_evaluator_runtime_error_nuon_format() {
        let engine_state = nu_cmd_lang::create_default_context();
        let evaluator = Evaluator::new(engine_state);

        let result = evaluator.eval(
            r#"error make {msg: "custom runtime error" label: {text: "problem here" span: {start: 0 end: 5}}}"#,
        );

        assert!(result.is_err());
        let err = result.unwrap_err();
        let err_msg = err.message.to_string();

        // Error should be in NUON format with structured fields
        assert!(
            err_msg.contains("msg:") && err_msg.contains("custom runtime error"),
            "Error message should contain msg field with custom error message, but got: {err_msg}"
        );

        assert!(
            err_msg.contains("code:"),
            "Error message should contain code field, but got: {err_msg}"
        );
    }

    #[test]
    fn test_closure_in_pipeline() {
        let engine_state = {
            let engine_state = nu_protocol::engine::EngineState::new();
            nu_cmd_lang::add_default_context(engine_state)
        };
        let evaluator = Evaluator::new(engine_state);

        let result = evaluator.eval(r#"do { |x| $x + 1 } 41"#);

        assert!(
            result.is_ok(),
            "Pipeline with closure should succeed: {:?}",
            result.err()
        );
        let output = result.unwrap();
        assert!(
            output.contains("42"),
            "Output should contain 42, got: {output}"
        );
    }

    #[test]
    fn test_repl_variable_persistence() {
        let engine_state = nu_cmd_lang::create_default_context();
        let evaluator = Evaluator::new(engine_state);

        let result = evaluator.eval("let x = 42");
        assert!(result.is_ok(), "Setting variable should succeed");

        let result = evaluator.eval("$x");
        assert!(
            result.is_ok(),
            "Variable should be accessible: {:?}",
            result.err()
        );
        let output = result.unwrap();
        assert!(
            output.contains("42"),
            "Variable $x should be 42, got: {output}"
        );
    }

    #[test]
    fn test_repl_env_persistence() {
        let engine_state = nu_cmd_lang::create_default_context();
        let evaluator = Evaluator::new(engine_state);

        let result = evaluator.eval("$env.TEST_VAR = 'hello_repl'");
        assert!(result.is_ok(), "Setting env var should succeed");

        let result = evaluator.eval("$env.TEST_VAR");
        assert!(
            result.is_ok(),
            "Env var should be accessible: {:?}",
            result.err()
        );
        let output = result.unwrap();
        assert!(
            output.contains("hello_repl"),
            "Env var should be 'hello_repl', got: {output}"
        );
    }

    #[test]
    fn test_history_ring_buffer() -> Result<(), Box<dyn std::error::Error>> {
        let engine_state = nu_cmd_lang::create_default_context();
        let evaluator = Evaluator::new(engine_state);

        // Set a small history limit
        evaluator.eval("$env.NU_MCP_HISTORY_LIMIT = 3")?;

        // Add items to history (the env var set above counts as first)
        // After limit=3 is set: history=[{set_result}]
        evaluator.eval("'second'")?; // history=[{set}, {second}]
        evaluator.eval("'third'")?; // history=[{set}, {second}, {third}] - at limit
        evaluator.eval("'fourth'")?; // evict oldest -> history=[{second}, {third}, {fourth}]
        evaluator.eval("'fifth'")?; // evict oldest -> history=[{third}, {fourth}, {fifth}]

        // At this point, before checking $history:
        // history = [{third}, {fourth}, {fifth}]
        // $history.0 should be "third"
        let result = evaluator.eval("$history.0")?;
        assert!(
            result.contains("third"),
            "Oldest entry should be 'third' after eviction, got: {result}"
        );

        // After the above query, history was:
        // evict oldest -> [{fourth}, {fifth}]
        // append result -> [{fourth}, {fifth}, {result_of_query}]
        // So now $history.1 = "fifth"
        let result = evaluator.eval("$history.1")?;
        assert!(
            result.contains("fifth"),
            "Entry at index 1 should be 'fifth', got: {result}"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_cancellation_discards_state() {
        let engine_state = nu_cmd_lang::create_default_context();
        let evaluator = Evaluator::new(engine_state);

        // Set a variable first
        evaluator
            .eval_async("let x = 1", CancellationToken::new())
            .await
            .unwrap();

        // Start an evaluation that we'll cancel
        let ct = CancellationToken::new();
        let ct_clone = ct.clone();

        // Cancel immediately
        ct_clone.cancel();

        // This should be cancelled and state should not change
        let result = evaluator.eval_async("let x = 999", ct).await;
        assert!(result.is_err(), "Cancelled evaluation should error");

        // Original variable should still be 1
        let result = evaluator
            .eval_async("$x", CancellationToken::new())
            .await
            .unwrap();
        assert!(
            result.contains('1') && !result.contains("999"),
            "Variable should still be 1 after cancelled eval, got: {result}"
        );
    }
}
