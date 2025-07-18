use std::sync::Arc;

use nu_protocol::engine::{Command, EngineState};
use rmcp::{handler::server::{router::tool::IntoToolRoute, tool::ToolRouter}, model::{ListToolsResult, PaginatedRequestParam, ServerCapabilities, ServerInfo, Tool}, service::RequestContext, tool_handler, tool_router, RoleServer, ServerHandler};

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

fn command_to_tool(command: &dyn Command) -> Tool {
    Tool {
        name: command.name().into(),
        description: Some(command.description().into()),
        input_schema: command.input_schema().clone(),
        output_schema: command.output_schema().clone(),
        annotations: None,
    }
}

