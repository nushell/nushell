use std::{borrow::Cow, ops::Range, sync::Arc};

use crate::shell_error_to_mcp_error;
use moka::sync::Cache;
use nu_engine::eval_block;
use nu_parser::parse;
use nu_protocol::{
    PipelineData, PipelineExecutionData, Value,
    debugger::WithoutDebug,
    engine::{EngineState, Stack, StateWorkingSet},
    write_all_and_flush,
};
use rmcp::model::Content;
use rmcp::{ErrorData as McpError, model::RawContent};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

const MAX_PAGE_SIZE: usize = 16 * 1024;

pub struct Evaluator {
    engine_state: Arc<EngineState>,
    cache: Cache<String, Arc<PipelineBuffer>>,
}

impl Evaluator {
    pub fn new(engine_state: Arc<EngineState>) -> Self {
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
            let engine_state = Arc::clone(&self.engine_state);
            let mut working_set = StateWorkingSet::new(&engine_state);

            // Parse the source code
            let block = parse(&mut working_set, None, nu_source.as_bytes(), false);

            // Check for parse errors
            if !working_set.parse_errors.is_empty() {
                // ShellError doesn't have ParseError, use LabeledError to contain it.
                return Err(McpError::invalid_request(
                    "Failed to parse nushell pipeline",
                    None,
                ));
            }

            // Eval the block with the input
            let mut stack = Stack::new().collect_value();
            let output = eval_block::<WithoutDebug>(
                &engine_state,
                &mut stack,
                &block,
                PipelineData::empty(),
            )
            .map_err(shell_error_to_mcp_error)?;

            let r = Arc::new(self.process_pipeline(output)?);
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
    ) -> Result<PipelineBuffer, McpError> {
        let engine_state = Arc::clone(&self.engine_state);
        let span = pipeline_execution_data.span();
        // todo - this bystream use case won't work
        if let PipelineData::ByteStream(_stream, ..) = pipeline_execution_data.body {
            // Copy ByteStreams directly
            // stream.print(false)
            Err(McpError::internal_error(
                Cow::from("ByteStream output is not supported"),
                None,
            ))
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
                    return Err(shell_error_to_mcp_error(*error));
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
                    .map_err(shell_error_to_mcp_error)?;
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
        let evaluator = Evaluator::new(Arc::new(engine_state));
        let result = evaluator.eval(&nuon_values, None)?;
        assert_eq!(result.summary.total, 3);
        assert!(result.summary.next_cursor.is_some());
        Ok(())
    }
}
