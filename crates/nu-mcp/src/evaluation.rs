use std::{borrow::Cow, sync::Arc};

use crate::shell_error_to_mcp_error;
use nu_engine::eval_block;
use nu_parser::parse;
use nu_protocol::{
    debugger::WithoutDebug, engine::{EngineState, Stack, StateWorkingSet}, write_all_and_flush, PipelineData, PipelineExecutionData, Value
};
use rmcp::{model::{Annotated, RawContent}, ErrorData as McpError};
use rmcp::model::Content;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct Evaluator {
    engine_state: Arc<EngineState>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct EvalResult(Vec<Content>);

impl Evaluator {
    pub fn new(engine_state: Arc<EngineState>) -> Self {
        Self { engine_state }
    }

    pub fn eval(&self, nu_source: &str, input: PipelineData) -> Result<EvalResult, McpError> {
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
        let output = eval_block::<WithoutDebug>(&engine_state, &mut stack, &block, input)
            .map_err(shell_error_to_mcp_error)?;

        self.pipeline_to_content(output)
            .inspect(|c| println!("--- content conversion stg 1: {c:#?}"))
            .map(|content| vec![content])
            .inspect(|c| println!("--- content conversion stg 2: {c:#?}"))
            .map(EvalResult)
            .map_err(|e| McpError::internal_error(format!("Failed to evaluate block: {e}"), None))
    }

    fn pipeline_to_content(
        &self,
        pipeline_execution_data: PipelineExecutionData,
    ) -> Result<Content, McpError> {
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
            let mut buffer: Vec<u8> = Vec::new();
            let config = engine_state.get_config();
            for item in pipeline_execution_data.body {
                let out = if let Value::Error { error, .. } = item {
                    return Err(shell_error_to_mcp_error(*error));
                } else {
                    item.to_expanded_string("\n", config)
                };

                write_all_and_flush(out, &mut buffer, "mcp_output", span, engine_state.signals())
                    .map_err(shell_error_to_mcp_error)?;
            }
            let content = RawContent::text(String::from_utf8(buffer).map_err(|e| {
                McpError::internal_error(format!("Invalid UTF-8 output: {e}"), None)
            })?);
            Ok(Annotated::new(content, None))
        }
    }
}
