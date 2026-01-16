use crate::{history::History, shell_error_to_mcp_error};
use nu_protocol::{
    PipelineData, PipelineExecutionData, Span, Value,
    debugger::WithoutDebug,
    engine::{EngineState, Stack, StateWorkingSet},
};
use std::{
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

const OUTPUT_LIMIT_ENV_VAR: &str = "NU_MCP_OUTPUT_LIMIT";
const DEFAULT_OUTPUT_LIMIT: usize = 10 * 1024; // 10kb

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

struct EvalState {
    engine_state: EngineState,
    stack: Stack,
    history: History,
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
                stack: Stack::new(),
                history,
            }),
        }
    }

    pub fn eval(&self, nu_source: &str) -> Result<String, rmcp::ErrorData> {
        let mut state = self.state.lock().expect("evaluator lock poisoned");

        let EvalState {
            engine_state,
            stack,
            history,
        } = &mut *state;

        let (block, delta) = {
            let mut working_set = StateWorkingSet::new(engine_state);
            let block = nu_parser::parse(&mut working_set, None, nu_source.as_bytes(), false);

            if let Some(err) = working_set.parse_errors.first() {
                return Err(rmcp::ErrorData::internal_error(
                    nu_protocol::format_cli_error(None, &working_set, err, None),
                    None,
                ));
            }

            if let Some(err) = working_set.compile_errors.first() {
                return Err(rmcp::ErrorData::internal_error(
                    nu_protocol::format_cli_error(None, &working_set, err, None),
                    None,
                ));
            }

            (block, working_set.render())
        };

        engine_state
            .merge_delta(delta)
            .map_err(|e| shell_error_to_mcp_error(e, engine_state))?;

        // Set up $history variable on the stack before evaluation
        stack.add_var(history.var_id(), history.as_value());

        let output = nu_engine::eval_block::<WithoutDebug>(
            engine_state,
            stack,
            &block,
            PipelineData::empty(),
        )
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
        let mut buffer = Vec::new();
        stream
            .write_to(&mut buffer)
            .map_err(|e| shell_error_to_mcp_error(e, engine_state))?;
        let string_output = String::from_utf8_lossy(&buffer).into_owned();
        let value = Value::string(&string_output, Span::unknown());
        return Ok((value, string_output));
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
    fn test_evaluator_parse_error_message() {
        let engine_state = nu_cmd_lang::create_default_context();
        let evaluator = Evaluator::new(engine_state);

        let result = evaluator.eval("let x = [1, 2, 3");

        assert!(result.is_err());
        let err = result.unwrap_err();
        let err_msg = err.message.to_string();

        assert!(
            err_msg.contains("Error: nu::parser::") && err_msg.contains("unexpected_eof"),
            "Error message should contain error code 'nu::parser::unexpected_eof', but got: {err_msg}"
        );

        assert!(
            err_msg.contains("let x = [1, 2, 3"),
            "Error message should contain source code context, but got: {err_msg}"
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
    fn test_evaluator_compile_error_message() {
        let engine_state = nu_cmd_lang::create_default_context();
        let evaluator = Evaluator::new(engine_state);

        let result = evaluator.eval("[{a: 1}] | get a");

        assert!(result.is_err());
        let err = result.unwrap_err();
        let err_msg = err.message.to_string();

        assert!(
            err_msg.contains("Error: nu::compile::"),
            "Error message should contain error code 'nu::compile::', but got: {err_msg}"
        );

        assert!(
            !err_msg.contains("Span {"),
            "Error message should not contain raw Debug formatting, but got: {err_msg}"
        );
    }

    #[test]
    fn test_evaluator_runtime_error_message() {
        let engine_state = nu_cmd_lang::create_default_context();
        let evaluator = Evaluator::new(engine_state);

        let result = evaluator.eval(
            r#"error make {msg: "custom runtime error" label: {text: "problem here" span: {start: 0 end: 5}}}"#,
        );

        assert!(result.is_err());
        let err = result.unwrap_err();
        let err_msg = err.message.to_string();

        assert!(
            err_msg.contains("Error:") && err_msg.contains("custom runtime error"),
            "Error message should contain rich formatting and custom error message, but got: {err_msg}"
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

    /// Create an engine state with all commands and proper environment setup for external commands.
    fn create_full_engine_state_for_external_commands() -> nu_protocol::engine::EngineState {
        let mut engine_state = nu_protocol::engine::EngineState::new();
        engine_state = nu_cmd_lang::add_default_context(engine_state);
        engine_state = nu_command::add_shell_command_context(engine_state);

        // Set up PWD environment variable (required for external commands)
        let cwd = std::env::current_dir().expect("Could not get current working directory.");
        engine_state.add_env_var("PWD".into(), Value::test_string(cwd.to_string_lossy()));

        // Set up PATH from the current environment
        if let Ok(path) = std::env::var("PATH") {
            engine_state.add_env_var("PATH".into(), Value::test_string(path));
        }

        engine_state
    }

    /// Test that the `complete` command works with external commands that return non-zero exit codes.
    ///
    /// This test verifies that `complete` correctly captures stdout, stderr, and exit_code
    /// from external commands without the error propagating.
    #[test]
    fn test_complete_command_with_external_error() {
        let engine_state = create_full_engine_state_for_external_commands();
        let evaluator = Evaluator::new(engine_state);

        // Use /bin/ls directly to avoid PATH issues, with a path that definitely doesn't exist
        // This should work - the `complete` command should capture the non-zero exit code
        // instead of propagating an error
        let result = evaluator.eval("do { ^/bin/ls /nonexistent_path_12345 } | complete");

        assert!(
            result.is_ok(),
            "complete should capture external command errors, got: {:?}",
            result.err()
        );

        let output = result.unwrap();
        assert!(
            output.contains("exit_code"),
            "Output should contain exit_code field, got: {output}"
        );
    }

    /// Test that piping an external command directly to `complete` works.
    ///
    /// This is the bug reported in https://github.com/nushell/nushell/issues/17173
    /// In MCP mode, `^ls nonexistent | complete` throws an error instead of capturing it.
    #[test]
    fn test_complete_command_direct_pipe() {
        let engine_state = create_full_engine_state_for_external_commands();
        let evaluator = Evaluator::new(engine_state);

        // Use /bin/ls directly to avoid PATH issues
        // This is the problematic case - direct piping without `do { }`
        let result = evaluator.eval("^/bin/ls /nonexistent_path_12345 | complete");

        assert!(
            result.is_ok(),
            "complete should capture external command errors even with direct piping, got: {:?}",
            result.err()
        );

        let output = result.unwrap();
        assert!(
            output.contains("exit_code"),
            "Output should contain exit_code field, got: {output}"
        );
    }

    /// Test the exact command from issue #17173: `let a = (^ls nonexistent | complete); $a`
    ///
    /// This tests the specific case where the result is assigned to a variable.
    #[test]
    fn test_complete_command_with_variable_assignment() {
        let engine_state = create_full_engine_state_for_external_commands();
        let evaluator = Evaluator::new(engine_state);

        // This is the exact command from the bug report
        let result = evaluator.eval("let a = (^/bin/ls /nonexistent_path_12345 | complete); $a");

        assert!(
            result.is_ok(),
            "complete with variable assignment should work, got: {:?}",
            result.err()
        );

        let output = result.unwrap();
        assert!(
            output.contains("exit_code"),
            "Output should contain exit_code field, got: {output}"
        );
    }

    /// Test using the NushellMcpServer to see if the issue is in the server setup.
    /// This mimics the actual MCP server configuration.
    #[test]
    fn test_complete_command_via_mcp_server_style_setup() {
        use nu_protocol::UseAnsiColoring;
        use std::sync::Arc;

        // Create engine state exactly like the MCP server does (from NushellMcpServer::new)
        let mut engine_state = nu_protocol::engine::EngineState::new();
        engine_state = nu_cmd_lang::add_default_context(engine_state);
        engine_state = nu_command::add_shell_command_context(engine_state);

        // Set up PWD environment variable
        let cwd = std::env::current_dir().expect("Could not get current working directory.");
        engine_state.add_env_var("PWD".into(), Value::test_string(cwd.to_string_lossy()));

        // Set up PATH
        if let Ok(path) = std::env::var("PATH") {
            engine_state.add_env_var("PATH".into(), Value::test_string(path));
        }

        // Configure the engine state for MCP (from NushellMcpServer::new)
        if let Some(config) = Arc::get_mut(&mut engine_state.config) {
            config.use_ansi_coloring = UseAnsiColoring::False;
            config.color_config.clear();
        }

        let evaluator = Evaluator::new(engine_state);

        // This is the exact command from the bug report
        let result = evaluator.eval("let a = (^/bin/ls /nonexistent_path_12345 | complete); $a");

        assert!(
            result.is_ok(),
            "complete via MCP-style setup should work, got: {:?}",
            result.err()
        );

        let output = result.unwrap();
        assert!(
            output.contains("exit_code"),
            "Output should contain exit_code field, got: {output}"
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

    /// Integration test that spawns the actual worker binary and tests the complete command.
    /// This helps isolate if the issue is in the binary setup vs the Evaluator directly.
    #[test]
    fn test_complete_via_spawned_worker() {
        use std::io::{BufRead, BufReader, Write};
        use std::os::unix::net::UnixStream;
        use std::process::{Command, Stdio};

        // Build the binary first (this should be a no-op if already built)
        let cargo_build = Command::new("cargo")
            .args(["build", "--features", "mcp"])
            .current_dir(env!("CARGO_MANIFEST_DIR").to_owned() + "/../..")
            .output()
            .expect("Failed to build");
        assert!(cargo_build.status.success(), "cargo build failed");

        // Create a temp socket path
        let socket_path =
            std::env::temp_dir().join(format!("nu-mcp-test-{}.sock", std::process::id()));
        let _ = std::fs::remove_file(&socket_path);

        // Find the nu binary
        let nu_binary =
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/debug/nu");

        // Spawn the worker with inherited stdio (like normal process) and no config files
        let mut child = Command::new(&nu_binary)
            .arg("--no-config-file")
            .arg("--mcp-worker")
            .arg(&socket_path)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("Failed to spawn worker");

        // Wait for the socket to be ready
        for _ in 0..50 {
            if socket_path.exists() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        assert!(socket_path.exists(), "Worker socket not created");

        // Connect to the worker
        let stream = UnixStream::connect(&socket_path).expect("Failed to connect to worker");
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut writer = stream;

        // Send the complete command
        let request = serde_json::json!({
            "source": "let a = (^/bin/ls /nonexistent_test_path | complete); $a"
        });
        writeln!(writer, "{}", request).expect("Failed to write request");
        writer.flush().expect("Failed to flush");

        // Read the response
        let mut response_line = String::new();
        reader
            .read_line(&mut response_line)
            .expect("Failed to read response");

        // Parse the response
        let response: serde_json::Value =
            serde_json::from_str(&response_line).expect("Failed to parse response");

        // Check if it succeeded
        let result = response.get("result").expect("No result field");

        // Kill the worker
        let _ = child.kill();
        let _ = std::fs::remove_file(&socket_path);

        // The result should be Ok, not Err
        assert!(
            result.get("Ok").is_some(),
            "Expected Ok result, got: {:?}",
            result
        );

        let output = result.get("Ok").unwrap().as_str().unwrap();
        assert!(
            output.contains("exit_code"),
            "Output should contain exit_code, got: {output}"
        );
    }
}
