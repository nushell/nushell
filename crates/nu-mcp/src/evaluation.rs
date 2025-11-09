use std::{ops::Range, sync::Arc};

use crate::shell_error_to_mcp_error;
use moka::sync::Cache;
use nu_engine::eval_block;
use nu_parser::parse;
use nu_protocol::{
    Config, PipelineData, PipelineExecutionData, UseAnsiColoring, Value,
    debugger::WithoutDebug,
    engine::{EngineState, Stack, StateWorkingSet},
    write_all_and_flush,
};
use rmcp::model::Content;
use rmcp::{ErrorData as McpError, model::RawContent};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

const MAX_PAGE_SIZE: usize = 16 * 1024;

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
pub struct Evaluator {
    engine_state: EngineState,
    cache: Cache<String, Arc<PipelineBuffer>>,
}

impl Evaluator {
    pub fn new(mut engine_state: EngineState) -> Self {
        // Disable ANSI coloring for MCP - it's a computer-to-computer protocol
        let mut config = Config::clone(engine_state.get_config());
        config.use_ansi_coloring = UseAnsiColoring::False;
        engine_state.set_config(config);

        let cache = Cache::builder()
            .max_capacity(100)
            .time_to_live(std::time::Duration::from_secs(300))
            .build();
        Self {
            engine_state,
            cache,
        }
    }

    pub fn eval(&self, nu_source: &str, cursor: Option<usize>) -> Result<EvalResult, McpError> {
        let results: Arc<PipelineBuffer> = if let Some(pipeline_buffer) = self.cache.get(nu_source)
        {
            pipeline_buffer
        } else {
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

            // Eval the block with the input
            let mut stack = Stack::new().collect_value();
            let output = eval_block::<WithoutDebug>(
                &engine_state,
                &mut stack,
                &block,
                PipelineData::empty(),
            )
            .map_err(|e| shell_error_to_mcp_error(e, &engine_state))?;

            let r = Arc::new(self.process_pipeline(output, &engine_state)?);
            self.cache.insert(nu_source.to_string(), Arc::clone(&r));
            r
        };

        let cursor = cursor.unwrap_or(0);
        let next_cursor = if (cursor + 1) < results.pages.len() {
            Some(cursor + 1)
        } else {
            None
        };

        let (page_size, page_content) = results.get_page(cursor); // .map(|content| vec![content])
        Ok(EvalResult {
            results: page_content,
            summary: Summary {
                total: results.total,
                returned: page_size,
                next_cursor,
            },
        })
    }

    fn process_pipeline(
        &self,
        pipeline_execution_data: PipelineExecutionData,
        engine_state: &EngineState,
    ) -> Result<PipelineBuffer, McpError> {
        let span = pipeline_execution_data.span();
        // todo - this bystream use case won't work
        if let PipelineData::ByteStream(stream, ..) = pipeline_execution_data.body {
            let mut buffer: Vec<u8> = Vec::new();
            stream
                .write_to(&mut buffer)
                .map_err(|e| shell_error_to_mcp_error(e, engine_state))?;
            let buffer_len = buffer.len();

            Ok(PipelineBuffer {
                buffer,
                pages: vec![Page::new(0..buffer_len, 1)],
                total: 1,
            })
        } else {
            let config = engine_state.get_config();

            // This will pagenate by the number of
            // avalue entries per MAX_PAGES_SIZE
            let mut buffer: Vec<u8> = Vec::new();
            let mut total = 0;
            let mut page_total = 0;
            let mut pages: Vec<Page> = Vec::new();
            let mut last_index = 0;
            let mut last_page_index = 0;
            for item in pipeline_execution_data.body {
                let out = if let Value::Error { error, .. } = item {
                    return Err(shell_error_to_mcp_error(*error, engine_state));
                } else {
                    item.to_expanded_string("\n", config) + "\n"
                };

                // Check to see if we have exceeded our page size.
                // If we have mark the indexes and start a new page
                let current_length = last_page_index + out.len();
                if (current_length + buffer.len()) > MAX_PAGE_SIZE {
                    pages.push(Page::new(last_page_index..last_index, page_total));
                    last_page_index = last_index;
                    page_total = 0;
                }
                //increment totals
                total += 1;
                page_total += 1;
                last_index = buffer.len();

                write_all_and_flush(out, &mut buffer, "mcp_output", span, engine_state.signals())
                    .map_err(|e| shell_error_to_mcp_error(e, engine_state))?;
            }
            if pages.is_empty() {
                pages.push(Page::new(0..buffer.len(), page_total));
            }
            Ok(PipelineBuffer {
                buffer,
                pages,
                total,
            })
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct EvalResult {
    results: Content,
    summary: Summary,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct Summary {
    total: usize,
    returned: usize,
    next_cursor: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Page {
    range: Range<usize>,
    size: usize,
}

impl Page {
    fn new(range: Range<usize>, returned: usize) -> Self {
        Self {
            range,
            size: returned,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PipelineBuffer {
    buffer: Vec<u8>,
    pages: Vec<Page>,
    total: usize,
}

impl PipelineBuffer {
    fn get_page(&self, index: usize) -> (usize, Content) {
        self.pages
            .get(index)
            .map(|page| {
                let text = String::from_utf8_lossy(&self.buffer[page.range.clone()]).to_string();
                let raw_content = RawContent::text(text);
                (page.size, Content::new(raw_content, None))
            })
            .unwrap_or((0, Content::new(RawContent::text(""), None)))
    }
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
                        "text" => Value::string(lipsum::lipsum(MAX_PAGE_SIZE / 2),Span::test_data())
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
        let result = evaluator.eval(&nuon_values, None)?;
        assert_eq!(result.summary.total, 3);
        assert!(result.summary.next_cursor.is_some());
        Ok(())
    }

    #[test]
    fn test_evaluator_parse_error_message() {
        let engine_state = create_default_context();
        let evaluator = Evaluator::new(engine_state);

        // Invalid syntax - missing closing bracket
        let result = evaluator.eval("let x = [1, 2, 3", None);

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
        let result = evaluator.eval("[{a: 1}] | get a", None);

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
            None,
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
        let result = evaluator.eval(r#"do { |x| $x + 1 } 41"#, None);

        assert!(
            result.is_ok(),
            "Pipeline with closure should succeed: {:?}",
            result.err()
        );
        let output = result.unwrap();
        assert_eq!(output.summary.total, 1, "Should have 1 result");
    }
}
