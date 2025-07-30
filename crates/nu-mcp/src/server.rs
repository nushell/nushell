use std::{borrow::Cow, sync::Arc};

use nu_protocol::{
    PipelineData, Span,
    engine::{Call, EngineState, Stack},
};
use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::tool::ToolRouter,
    model::{ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
};

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
    async fn list_commands(&self) -> Result<String, McpError> {
        if let (Some(cmds_decl_id), Some(csv_decl_id)) = (
            self.engine_state.find_decl(b"help commands", &[]),
            self.engine_state.find_decl(b"to csv", &[]),
        ) {
            let mut stack = Stack::new();
            let span = Span::unknown();
            let call = Call::new(span);
            let pipeline = PipelineData::empty();
            let pipeline = self
                .engine_state
                .get_decl(cmds_decl_id)
                .run(self.engine_state.as_ref(), &mut stack, &call, pipeline)
                .map_err(shell_error_to_mcp_error)?;

            let (output, _span, _metadata) = self
                .engine_state
                .get_decl(csv_decl_id)
                .run(self.engine_state.as_ref(), &mut stack, &call, pipeline)
                .and_then(|data| data.collect_string_strict(span))
                .map_err(shell_error_to_mcp_error)?;
            Ok(output)
        } else {
            return Err(McpError::resource_not_found(
                Cow::from("Either nushell command 'help commands' or 'to csv' not available"),
                None,
            ));
        }
    }
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
    McpError::internal_error(format!("{error}"), None)
}
