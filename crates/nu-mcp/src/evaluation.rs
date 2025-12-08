use std::sync::Mutex;

use crate::shell_error_to_mcp_error;
use nu_engine::eval_block;
use nu_parser::parse;
use nu_protocol::{
    Config, PipelineData, PipelineExecutionData, Span, UseAnsiColoring, Value, VarId, record,
    debugger::WithoutDebug,
    engine::{EngineState, Stack, StateWorkingSet},
};
use rmcp::ErrorData as McpError;

const DEFAULT_OUTPUT_TRUNCATE_THRESHOLD: usize = 10_000;
const OUTPUT_TRUNCATE_ENV_VAR: &str = "NU_MCP_OUTPUT_TRUNCATE";

/// Evaluates Nushell code in isolated contexts for MCP.
///
/// # Architecture
///
/// The evaluator maintains a pristine `EngineState` template. Each evaluation:
/// 1. Clones the engine state (cheap due to internal `Arc`s)
/// 2. Parses code into a `Block` and gets a `StateDelta` via `working_set.render()`
/// 3. **Merges the delta** via `engine_state.merge_delta()` to register blocks
/// 4. Evaluates the block with the merged state
///
/// Step 3 is critical: parsed blocks (including closures) are only stored in the
/// `StateWorkingSet` initially. Without merging, `eval_block()` will panic with
/// "missing block" when it tries to execute closures or other block references.
///
/// # Isolation
///
/// Each evaluation gets its own cloned state, so variables/definitions from one
/// evaluation don't persist to the next.
///
/// This architecture also enables future parallel evaluation of multiple pipelines.
///
/// # History
///
/// The evaluator maintains a `$history` list that stores all command outputs.
/// Each evaluation can access previous outputs via `$history.0`, `$history.1`, etc.
/// Large outputs are truncated in the response but stored in full in history.
pub struct Evaluator {
    engine_state: EngineState,
    history: Mutex<Vec<Value>>,
    history_var_id: VarId,
}

impl Evaluator {
    pub fn new(mut engine_state: EngineState) -> Self {
        // Disable ANSI coloring for MCP - it's a computer-to-computer protocol
        let mut config = Config::clone(engine_state.get_config());
        config.use_ansi_coloring = UseAnsiColoring::False;
        engine_state.set_config(config);

        // Register the $history variable in the engine state
        let history_var_id = {
            let mut working_set = StateWorkingSet::new(&engine_state);
            let var_id = working_set.add_variable(
                b"history".to_vec(),
                Span::unknown(),
                nu_protocol::Type::List(Box::new(nu_protocol::Type::Any)),
                false,
            );
            let delta = working_set.render();
            engine_state
                .merge_delta(delta)
                .expect("failed to register $history variable");
            var_id
        };

        Self {
            engine_state,
            history: Mutex::new(Vec::new()),
            history_var_id,
        }
    }

    fn output_truncate_threshold() -> usize {
        std::env::var(OUTPUT_TRUNCATE_ENV_VAR)
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_OUTPUT_TRUNCATE_THRESHOLD)
    }

    pub fn eval(&self, nu_source: &str) -> Result<String, McpError> {
        // Clone the pristine engine state for this evaluation
        let mut engine_state = self.engine_state.clone();

        let (block, delta) = {
            let mut working_set = StateWorkingSet::new(&engine_state);

            // Parse the source code
            let block = parse(&mut working_set, None, nu_source.as_bytes(), false);

            // Check for parse errors
            if let Some(err) = working_set.parse_errors.first() {
                return Err(McpError::internal_error(
                    nu_protocol::format_cli_error(&working_set, err, None),
                    None,
                ));
            }

            // Check for compile errors (IR compilation errors)
            // These are caught during the parse/compile phase, before evaluation
            if let Some(err) = working_set.compile_errors.first() {
                return Err(McpError::internal_error(
                    nu_protocol::format_cli_error(&working_set, err, None),
                    None,
                ));
            }

            (block, working_set.render())
        };

        // Merge the parsed blocks into the engine state so they're available during eval
        engine_state
            .merge_delta(delta)
            .map_err(|e| shell_error_to_mcp_error(e, &engine_state))?;

        // Set up the stack with $history variable
        let mut stack = Stack::new().collect_value();
        {
            let history_guard = self.history.lock().expect("history mutex poisoned");
            let history_value = Value::list(history_guard.clone(), Span::unknown());
            stack.add_var(self.history_var_id, history_value);
        }

        // Eval the block with the input
        let output =
            eval_block::<WithoutDebug>(&engine_state, &mut stack, &block, PipelineData::empty())
                .map_err(|e| shell_error_to_mcp_error(e, &engine_state))?;

        // Get cwd after evaluation (command may have changed it)
        let cwd = engine_state
            .cwd(Some(&stack))
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|_| String::from("unknown"));

        // Process the output value
        let (output_value, output_nuon) = self.process_pipeline(output, &engine_state)?;

        // Store the output in history and get its index
        let history_index = {
            let mut history_guard = self.history.lock().expect("history mutex poisoned");
            let idx = history_guard.len();
            history_guard.push(output_value);
            idx
        };

        // Check if output needs truncation
        let threshold = Self::output_truncate_threshold();
        let final_output = if output_nuon.len() > threshold {
            format!("(output truncated, full result saved to $history.{})", history_index)
        } else {
            output_nuon
        };

        // Wrap response in a structured record
        let response = Value::record(
            record! {
                "history_index" => Value::int(history_index as i64, Span::unknown()),
                "cwd" => Value::string(cwd, Span::unknown()),
                "output" => Value::string(final_output, Span::unknown()),
            },
            Span::unknown(),
        );

        nuon::to_nuon(
            &engine_state,
            &response,
            nuon::ToStyle::Raw,
            Some(Span::unknown()),
            false,
        )
        .map_err(|e| shell_error_to_mcp_error(e, &engine_state))
    }

    fn process_pipeline(
        &self,
        pipeline_execution_data: PipelineExecutionData,
        engine_state: &EngineState,
    ) -> Result<(Value, String), McpError> {
        let span = pipeline_execution_data.span();

        if let PipelineData::ByteStream(stream, ..) = pipeline_execution_data.body {
            let mut buffer: Vec<u8> = Vec::new();
            stream
                .write_to(&mut buffer)
                .map_err(|e| shell_error_to_mcp_error(e, engine_state))?;
            let string_output = String::from_utf8_lossy(&buffer).into_owned();
            let value = Value::string(&string_output, Span::unknown());
            Ok((value, string_output))
        } else {
            // Collect all values from the pipeline
            let mut values: Vec<Value> = Vec::new();
            for item in pipeline_execution_data.body {
                if let Value::Error { error, .. } = &item {
                    return Err(shell_error_to_mcp_error(*error.clone(), engine_state));
                }
                values.push(item);
            }

            // Convert the entire result to NUON format
            // If there's a single value, output it directly; otherwise wrap in a list
            let value_to_store = match values.len() {
                1 => values
                    .pop()
                    .expect("values has exactly one element; this cannot fail"),
                _ => Value::list(values, span.unwrap_or(Span::unknown())),
            };

            let nuon_string = nuon::to_nuon(
                engine_state,
                &value_to_store,
                nuon::ToStyle::Raw,
                Some(Span::unknown()),
                false,
            )
            .map_err(|e| shell_error_to_mcp_error(e, engine_state))?;

            Ok((value_to_store, nuon_string))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nu_cmd_lang::create_default_context;

    #[test]
    fn test_evaluator_response_format() -> Result<(), Box<dyn std::error::Error>> {
        let engine_state = create_default_context();
        let evaluator = Evaluator::new(engine_state);
        let result = evaluator.eval("42")?;

        // Response should be wrapped in a record with history_index, cwd, and output
        assert!(
            result.contains("history_index"),
            "Response should contain history_index, got: {result}"
        );
        assert!(
            result.contains("cwd"),
            "Response should contain cwd, got: {result}"
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
        let engine_state = create_default_context();
        let evaluator = Evaluator::new(engine_state);

        let result1 = evaluator.eval("1")?;
        let result2 = evaluator.eval("2")?;
        let result3 = evaluator.eval("3")?;

        // First command should have history_index: 0 (NUON format has no space after colon)
        assert!(
            result1.contains("history_index:0") || result1.contains("history_index: 0"),
            "First result should have history_index: 0, got: {result1}"
        );
        // Second command should have history_index: 1
        assert!(
            result2.contains("history_index:1") || result2.contains("history_index: 1"),
            "Second result should have history_index: 1, got: {result2}"
        );
        // Third command should have history_index: 2
        assert!(
            result3.contains("history_index:2") || result3.contains("history_index: 2"),
            "Third result should have history_index: 2, got: {result3}"
        );
        Ok(())
    }

    #[test]
    fn test_history_variable_exists() -> Result<(), Box<dyn std::error::Error>> {
        let engine_state = create_default_context();
        let evaluator = Evaluator::new(engine_state);

        // First command stores a value
        evaluator.eval("42")?;

        // Second command just accesses history (returns empty list initially since we can't use length)
        // This tests that $history is accessible as a variable
        let result = evaluator.eval("$history")?;

        // Should succeed and contain the output
        assert!(
            result.contains("output"),
            "Should be able to access $history, got: {result}"
        );
        // The output should contain the first value (42)
        assert!(
            result.contains("42"),
            "History should contain previous output 42, got: {result}"
        );
        Ok(())
    }

    #[test]
    fn test_history_access_by_index() -> Result<(), Box<dyn std::error::Error>> {
        let engine_state = create_default_context();
        let evaluator = Evaluator::new(engine_state);

        // Store some values
        evaluator.eval("100")?;
        evaluator.eval("200")?;

        // Access the first history item
        let result = evaluator.eval("$history.0")?;
        assert!(
            result.contains("100"),
            "history.0 should be 100, got: {result}"
        );

        // Access the second history item
        let result = evaluator.eval("$history.1")?;
        assert!(
            result.contains("200"),
            "history.1 should be 200, got: {result}"
        );

        Ok(())
    }

    #[test]
    fn test_output_truncation() -> Result<(), Box<dyn std::error::Error>> {
        // Set a very low threshold for testing
        // SAFETY: This test runs serially (cargo nextest) and we restore the var after
        unsafe { std::env::set_var(OUTPUT_TRUNCATE_ENV_VAR, "20") };

        let engine_state = create_default_context();
        let evaluator = Evaluator::new(engine_state);

        // Generate output larger than 20 chars using just a long string literal
        let result = evaluator.eval("\"this is a very long string that exceeds the truncation threshold for testing purposes\"")?;

        // Should be truncated with a message about history
        assert!(
            result.contains("output truncated") && result.contains("$history"),
            "Large output should be truncated, got: {result}"
        );

        // Clean up
        // SAFETY: restoring env var state
        unsafe { std::env::remove_var(OUTPUT_TRUNCATE_ENV_VAR) };
        Ok(())
    }

    #[test]
    fn test_evaluator_parse_error_message() {
        let engine_state = create_default_context();
        let evaluator = Evaluator::new(engine_state);

        // Invalid syntax - missing closing bracket
        let result = evaluator.eval("let x = [1, 2, 3");

        assert!(result.is_err());
        let err = result.unwrap_err();
        let err_msg = err.message.to_string();

        // Should contain rich error formatting with error code and labeled spans
        assert!(
            err_msg.contains("Error: nu::parser::") && err_msg.contains("unexpected_eof"),
            "Error message should contain error code 'nu::parser::unexpected_eof', but got: {err_msg}"
        );

        // Should contain source code context
        assert!(
            err_msg.contains("let x = [1, 2, 3"),
            "Error message should contain source code context, but got: {err_msg}"
        );

        // Should NOT contain ANSI escape codes (starts with ESC character '\x1b[')
        assert!(
            !err_msg.contains('\x1b'),
            "Error message should not contain ANSI escape codes, but got: {err_msg:?}"
        );

        // Should NOT contain Debug formatting like Span { start: ... }
        assert!(
            !err_msg.contains("Span {"),
            "Error message should not contain raw Debug formatting, but got: {err_msg}"
        );
    }

    #[test]
    fn test_evaluator_compile_error_message() {
        let engine_state = create_default_context();
        let evaluator = Evaluator::new(engine_state);

        // This will trigger a compile error (IR compilation)
        // because create_default_context doesn't fully compile blocks for pipelines
        let result = evaluator.eval("[{a: 1}] | get a");

        assert!(result.is_err());
        let err = result.unwrap_err();
        let err_msg = err.message.to_string();

        // Should contain rich error formatting with error code
        assert!(
            err_msg.contains("Error: nu::compile::"),
            "Error message should contain error code 'nu::compile::', but got: {err_msg}"
        );

        // Should NOT contain Debug formatting like Span { start: ... }
        assert!(
            !err_msg.contains("Span {"),
            "Error message should not contain raw Debug formatting, but got: {err_msg}"
        );
    }

    #[test]
    fn test_evaluator_runtime_error_message() {
        let engine_state = create_default_context();
        let evaluator = Evaluator::new(engine_state);

        // Use error make to create a runtime error with custom message and labels
        let result = evaluator.eval(
            r#"error make {msg: "custom runtime error" label: {text: "problem here" span: {start: 0 end: 5}}}"#,
        );

        assert!(result.is_err());
        let err = result.unwrap_err();
        let err_msg = err.message.to_string();

        // Should contain:
        // 1. Rich error formatting with "Error:" prefix
        // 2. The custom error message
        // 3. NOT just a generic "ShellError: ..." message
        assert!(
            err_msg.contains("Error:") && err_msg.contains("custom runtime error"),
            "Error message should contain rich formatting and custom error message, but got: {err_msg}"
        );
    }

    #[test]
    fn test_closure_in_pipeline() {
        // Use add_default_context which includes basic language commands
        let engine_state = {
            let engine_state = nu_protocol::engine::EngineState::new();
            nu_cmd_lang::add_default_context(engine_state)
        };
        let evaluator = Evaluator::new(engine_state);

        // Test with a simple closure using 'do' which is a lang command
        // The closure { ... } creates a block that must be available when eval_block runs
        // This tests that merge_delta properly registers blocks in the engine_state
        let result = evaluator.eval(r#"do { |x| $x + 1 } 41"#);

        assert!(
            result.is_ok(),
            "Pipeline with closure should succeed: {:?}",
            result.err()
        );
        let output = result.unwrap();
        // Now output is wrapped in a record, so check for the value in the output field
        assert!(
            output.contains("42"),
            "Output should contain 42, got: {output}"
        );
    }
}
