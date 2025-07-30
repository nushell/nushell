use std::{borrow::Cow, sync::Arc};

use nu_protocol::{
    PipelineData, Span, Value,
    engine::{Call, EngineState, Stack},
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
    engine_state: Arc<EngineState>,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl NushellMcpServer {
    pub fn new(engine_state: EngineState) -> Self {
        NushellMcpServer {
            engine_state: Arc::new(engine_state),
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "List available commands")]
    async fn list_commands(&self) -> Result<CallToolResult, McpError> {
        execute_cmd(
            self.engine_state.as_ref(),
            PipelineData::empty(),
            b"help commands",
        )
        .and_then(|pipeline| execute_cmd(self.engine_state.as_ref(), pipeline, b"to json"))
        .and_then(|pipeline| pipeline_to_content(pipeline, self.engine_state.as_ref()))
        .map(|content| CallToolResult::success(vec![content]))
    }

    #[tool(description = "Get help for a specific command")]
    async fn command_help(
        &self,
        Parameters(CommandNameRequest { name }): Parameters<CommandNameRequest>,
    ) -> Result<CallToolResult, McpError> {
        let cmd = format!("help {name}");
        execute_cmd(
            self.engine_state.as_ref(),
            PipelineData::empty(),
            cmd.as_bytes(),
        )
        .and_then(|pipeline| pipeline_to_content(pipeline, self.engine_state.as_ref()))
        .map(|content| CallToolResult::success(vec![content]))
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
struct CommandNameRequest {
    name: String,
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

fn execute_cmd(
    engine_state: &EngineState,
    pipeline_data: PipelineData,
    name: &[u8],
) -> Result<PipelineData, McpError> {
    let mut stack = Stack::new();
    let span = Span::unknown();
    let call = Call::new(span);
    if let Some(decl_id) = engine_state.find_decl(name, &[]) {
        engine_state
            .get_decl(decl_id)
            .run(engine_state, &mut stack, &call, pipeline_data)
            .map_err(shell_error_to_mcp_error)
    } else {
        Err(McpError::resource_not_found(
            Cow::from("Command not found"),
            None,
        ))
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
