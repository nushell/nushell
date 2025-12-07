use crate::shell_error_to_mcp_error;
use nu_engine::eval_block;
use nu_parser::parse;
use nu_protocol::{
    Config, PipelineData, PipelineExecutionData, Span, UseAnsiColoring, Value,
    debugger::WithoutDebug,
    engine::{EngineState, Stack, StateWorkingSet},
};
use rmcp::ErrorData as McpError;

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
pub struct Evaluator {
    state: std::sync::Mutex<EvalState>,
}

struct EvalState {
    engine_state: EngineState,
    stack: Stack,
}

impl Evaluator {
    pub fn new(mut engine_state: EngineState) -> Self {
        // Disable ANSI coloring for MCP - it's a computer-to-computer protocol
        let mut config = Config::clone(engine_state.get_config());
        config.use_ansi_coloring = UseAnsiColoring::False;
        engine_state.set_config(config);

        Self {
            state: std::sync::Mutex::new(EvalState {
                engine_state,
                stack: Stack::new(),
            }),
        }
    }

    pub fn eval(&self, nu_source: &str) -> Result<String, McpError> {
        let mut state = self.state.lock().expect("evaluator lock poisoned");

        let (block, delta) = {
            let mut working_set = StateWorkingSet::new(&state.engine_state);

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
            if let Some(err) = working_set.compile_errors.first() {
                return Err(McpError::internal_error(
                    nu_protocol::format_cli_error(&working_set, err, None),
                    None,
                ));
            }

            (block, working_set.render())
        };

        // Destructure to satisfy the borrow checker
        let EvalState {
            engine_state,
            stack,
        } = &mut *state;

        // Merge the parsed blocks into the persistent engine state
        engine_state
            .merge_delta(delta)
            .map_err(|e| shell_error_to_mcp_error(e, engine_state))?;

        // Eval the block with persistent state and stack
        let output = eval_block::<WithoutDebug>(engine_state, stack, &block, PipelineData::empty())
            .map_err(|e| shell_error_to_mcp_error(e, engine_state))?;

        process_pipeline(output, engine_state)
    }
}

fn process_pipeline(
    pipeline_execution_data: PipelineExecutionData,
    engine_state: &EngineState,
) -> Result<String, McpError> {
    let span = pipeline_execution_data.span();

    if let PipelineData::ByteStream(stream, ..) = pipeline_execution_data.body {
        let mut buffer: Vec<u8> = Vec::new();
        stream
            .write_to(&mut buffer)
            .map_err(|e| shell_error_to_mcp_error(e, engine_state))?;
        return Ok(String::from_utf8_lossy(&buffer).into_owned());
    }

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
    let value_to_convert = match values.len() {
        1 => values
            .pop()
            .expect("values has exactly one element; this cannot fail"),
        _ => Value::list(values, span.unwrap_or(Span::unknown())),
    };

    nuon::to_nuon(
        engine_state,
        &value_to_convert,
        nuon::ToStyle::Raw,
        Some(Span::unknown()),
        false,
    )
    .map_err(|e| shell_error_to_mcp_error(e, engine_state))
}

#[cfg(test)]
mod tests {
    use super::*;
    use nu_cmd_lang::create_default_context;
    use nu_protocol::{Span, record};

    #[test]
    fn test_evaluator() -> Result<(), Box<dyn std::error::Error>> {
        let values: Vec<Value> = (0..3)
            .map(|index| {
                Value::record(
                    record! {
                        "index" => Value::int(index, Span::test_data()),
                        "text" => Value::string("hello", Span::test_data())
                    },
                    Span::test_data(),
                )
            })
            .collect();
        let values = Value::list(values, Span::test_data());
        let engine_state = create_default_context();

        let nuon_values = nuon::to_nuon(
            &engine_state,
            &values,
            nuon::ToStyle::Default,
            Some(Span::test_data()),
            false,
        )?;
        let evaluator = Evaluator::new(engine_state);
        let result = evaluator.eval(&nuon_values)?;
        // Result should be raw NUON
        assert!(result.contains("index"));
        assert!(result.contains("hello"));
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
        assert_eq!(output, "42", "Should return 42");
    }

    #[test]
    fn test_repl_variable_persistence() {
        let engine_state = create_default_context();
        let evaluator = Evaluator::new(engine_state);

        // Set a variable in first call
        let result = evaluator.eval("let x = 42");
        assert!(result.is_ok(), "Setting variable should succeed");

        // Access the variable in second call - should persist
        let result = evaluator.eval("$x");
        assert!(
            result.is_ok(),
            "Variable should be accessible: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_repl_env_persistence() {
        let engine_state = create_default_context();
        let evaluator = Evaluator::new(engine_state);

        // Set an env var in first call
        let result = evaluator.eval("$env.TEST_VAR = 'hello_repl'");
        assert!(result.is_ok(), "Setting env var should succeed");

        // Access the env var in second call - should persist
        let result = evaluator.eval("$env.TEST_VAR");
        assert!(
            result.is_ok(),
            "Env var should be accessible: {:?}",
            result.err()
        );
    }
}
