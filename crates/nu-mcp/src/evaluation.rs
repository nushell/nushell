use crate::shell_error_to_mcp_error;

const OUTPUT_LIMIT_ENV_VAR: &str = "NU_MCP_OUTPUT_LIMIT";

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
/// Large outputs are truncated in the response but stored in full in history.
pub struct Evaluator {
    state: std::sync::Mutex<EvalState>,
    history_var_id: nu_protocol::VarId,
}

struct EvalState {
    engine_state: nu_protocol::engine::EngineState,
    stack: nu_protocol::engine::Stack,
    history: Vec<nu_protocol::Value>,
}

impl Evaluator {
    pub fn new(mut engine_state: nu_protocol::engine::EngineState) -> Self {
        // Disable ANSI coloring for MCP - it's a computer-to-computer protocol
        let mut config = nu_protocol::Config::clone(engine_state.get_config());
        config.use_ansi_coloring = nu_protocol::UseAnsiColoring::False;
        engine_state.set_config(config);

        // Register the $history variable in the engine state
        let history_var_id = register_history_variable(&mut engine_state);

        Self {
            state: std::sync::Mutex::new(EvalState {
                engine_state,
                stack: nu_protocol::engine::Stack::new(),
                history: Vec::new(),
            }),
            history_var_id,
        }
    }

    pub fn eval(&self, nu_source: &str) -> Result<String, rmcp::ErrorData> {
        let mut state = self.state.lock().expect("evaluator lock poisoned");

        let (block, delta) = {
            let mut working_set = nu_protocol::engine::StateWorkingSet::new(&state.engine_state);

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

        // Destructure to satisfy the borrow checker
        let EvalState {
            engine_state,
            stack,
            history,
        } = &mut *state;

        engine_state
            .merge_delta(delta)
            .map_err(|e| shell_error_to_mcp_error(e, engine_state))?;

        // Set up $history variable on the stack before evaluation
        let history_value = nu_protocol::Value::list(history.clone(), nu_protocol::Span::unknown());
        stack.add_var(self.history_var_id, history_value);

        let output = nu_engine::eval_block::<nu_protocol::debugger::WithoutDebug>(
            engine_state,
            stack,
            &block,
            nu_protocol::PipelineData::empty(),
        )
        .map_err(|e| shell_error_to_mcp_error(e, engine_state))?;

        let cwd = engine_state
            .cwd(Some(stack))
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|_| String::from("unknown"));

        let (output_value, output_nuon) = process_pipeline(output, engine_state)?;

        let history_index = history.len();
        history.push(output_value);

        let should_truncate = output_limit(engine_state, stack)
            .is_some_and(|limit| output_nuon.len() > limit);
        let final_output = if should_truncate {
            format!(
                "(output truncated, full result saved to $history.{})",
                history_index
            )
        } else {
            output_nuon
        };

        let response = nu_protocol::Value::record(
            nu_protocol::record! {
                "history_index" => nu_protocol::Value::int(history_index as i64, nu_protocol::Span::unknown()),
                "cwd" => nu_protocol::Value::string(cwd, nu_protocol::Span::unknown()),
                "output" => nu_protocol::Value::string(final_output, nu_protocol::Span::unknown()),
            },
            nu_protocol::Span::unknown(),
        );

        nuon::to_nuon(
            engine_state,
            &response,
            nuon::ToStyle::Raw,
            Some(nu_protocol::Span::unknown()),
            false,
        )
        .map_err(|e| shell_error_to_mcp_error(e, engine_state))
    }
}

fn register_history_variable(
    engine_state: &mut nu_protocol::engine::EngineState,
) -> nu_protocol::VarId {
    let mut working_set = nu_protocol::engine::StateWorkingSet::new(engine_state);
    let var_id = working_set.add_variable(
        b"history".to_vec(),
        nu_protocol::Span::unknown(),
        nu_protocol::Type::List(Box::new(nu_protocol::Type::Any)),
        false,
    );
    let delta = working_set.render();
    engine_state
        .merge_delta(delta)
        .expect("failed to register $history variable");
    var_id
}

/// Returns the output limit in bytes, or `None` if not configured (no truncation).
fn output_limit(
    engine_state: &nu_protocol::engine::EngineState,
    stack: &nu_protocol::engine::Stack,
) -> Option<usize> {
    stack
        .get_env_var(engine_state, OUTPUT_LIMIT_ENV_VAR)
        .and_then(|v| v.as_filesize().ok())
        .and_then(|fs| usize::try_from(fs.get()).ok())
}

fn process_pipeline(
    pipeline_execution_data: nu_protocol::PipelineExecutionData,
    engine_state: &nu_protocol::engine::EngineState,
) -> Result<(nu_protocol::Value, String), rmcp::ErrorData> {
    let span = pipeline_execution_data.span();

    if let nu_protocol::PipelineData::ByteStream(stream, ..) = pipeline_execution_data.body {
        let mut buffer: Vec<u8> = Vec::new();
        stream
            .write_to(&mut buffer)
            .map_err(|e| shell_error_to_mcp_error(e, engine_state))?;
        let string_output = String::from_utf8_lossy(&buffer).into_owned();
        let value = nu_protocol::Value::string(&string_output, nu_protocol::Span::unknown());
        return Ok((value, string_output));
    }

    let mut values: Vec<nu_protocol::Value> = Vec::new();
    for item in pipeline_execution_data.body {
        if let nu_protocol::Value::Error { error, .. } = &item {
            return Err(shell_error_to_mcp_error(*error.clone(), engine_state));
        }
        values.push(item);
    }

    let value_to_store = match values.len() {
        1 => values
            .pop()
            .expect("values has exactly one element; this cannot fail"),
        _ => nu_protocol::Value::list(values, span.unwrap_or(nu_protocol::Span::unknown())),
    };

    let nuon_string = nuon::to_nuon(
        engine_state,
        &value_to_store,
        nuon::ToStyle::Raw,
        Some(nu_protocol::Span::unknown()),
        false,
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

        assert!(
            result.contains("output truncated") && result.contains("$history"),
            "Large output should be truncated when $env.NU_MCP_OUTPUT_LIMIT is set, got: {result}"
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
}
