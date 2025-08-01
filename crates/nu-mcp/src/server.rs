use std::{
    borrow::Cow,
    sync::{Mutex, MutexGuard},
};

use nu_engine::eval_block;
use nu_parser::parse;
use nu_protocol::{
    PipelineData, Value,
    debugger::WithoutDebug,
    engine::{EngineState, Stack, StateWorkingSet},
    write_all_and_flush,
};
use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::tool::{Parameters, ToolRouter},
    model::{Annotated, CallToolResult, Content, RawContent, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct NushellMcpServer {
    engine_state: Mutex<EngineState>,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl NushellMcpServer {
    pub fn new(engine_state: EngineState) -> Self {
        NushellMcpServer {
            engine_state: Mutex::new(engine_state),
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "List available Nushell commands")]
    async fn list_commands(&self) -> Result<CallToolResult, McpError> {
        self.eval("help commands | to json", PipelineData::empty())
            .map(CallToolResult::success)
    }

    #[tool(description = "Get help for a specific Nushell command")]
    async fn command_help(
        &self,
        Parameters(CommandNameRequest { name }): Parameters<CommandNameRequest>,
    ) -> Result<CallToolResult, McpError> {
        let cmd = format!("help {name}");
        self.eval(&cmd, PipelineData::empty())
            .map(CallToolResult::success)
    }

    #[tool(description = "Evaluate Nushell source code")]
    async fn evaluate(&self, Parameters(NuSourceRequest { input }): Parameters<NuSourceRequest>) -> Result<CallToolResult, McpError> {
        self.eval(&input, PipelineData::empty())
            .map(CallToolResult::success)
    }

    fn eval(&self, nu_source: &str, input: PipelineData) -> Result<Vec<Content>, McpError> {
        let mut engine_state = self.engine_state_lock()?;
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

        let rendered = working_set.render();

        // Merge into state
        engine_state
            .merge_delta(rendered)
            .map_err(shell_error_to_mcp_error)?;

        // Eval the block with the input
        let mut stack = Stack::new().collect_value();
        let output = eval_block::<WithoutDebug>(&mut engine_state, &mut stack, &block, input)
            .map_err(shell_error_to_mcp_error)?;

        pipeline_to_content(output, &engine_state)
            .map(|content| vec![content])
            .map_err(|e| McpError::internal_error(format!("Failed to evaluate block: {e}"), None))
    }

    fn engine_state_lock(&self) -> Result<MutexGuard<EngineState>, McpError> {
        self.engine_state.lock().map_err(|e| {
            McpError::internal_error(format!("Failed to acquire engine state lock: {e}"), None)
        })
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
struct CommandNameRequest {
    #[schemars(description = "The name of the command to get help for")]
    name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
struct NuSourceRequest {
    #[schemars(description = "The Nushell source code to evaluate")]
    input: String,
}

#[tool_handler]
impl ServerHandler for NushellMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("generic data service".into()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

fn shell_error_to_mcp_error(error: nu_protocol::ShellError) -> McpError {
    //todo - make this more sophisticated   :w
    McpError::internal_error(format!("ShellError: {error}"), None)
}

fn pipeline_to_content(
    pipeline_data: PipelineData,
    engine_state: &EngineState,
) -> Result<Content, McpError> {
    let span = pipeline_data.span();
    // todo - this bystream use case won't work
    if let PipelineData::ByteStream(_stream, ..) = pipeline_data {
        // Copy ByteStreams directly
        // stream.print(false)
        Err(McpError::internal_error(
            Cow::from("ByteStream output is not supported"),
            None,
        ))
    } else {
        let mut buffer: Vec<u8> = Vec::new();
        let config = engine_state.get_config();
        for item in pipeline_data {
            let out = if let Value::Error { error, .. } = item {
                return Err(shell_error_to_mcp_error(*error));
            } else {
                item.to_expanded_string("\n", config)
            };

            write_all_and_flush(out, &mut buffer, "mcp_output", span, engine_state.signals())
                .map_err(shell_error_to_mcp_error)?;
        }
        let content =
            RawContent::text(String::from_utf8(buffer).map_err(|e| {
                McpError::internal_error(format!("Invalid UTF-8 output: {e}"), None)
            })?);
        Ok(Annotated::new(content, None))
    }
}
