use std::sync::Arc;

use nu_protocol::{UseAnsiColoring, engine::EngineState};
use rmcp::{
    ErrorData as McpError, RoleServer, ServerHandler,
    handler::server::{tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    service::RequestContext,
    tool, tool_handler, tool_router,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::evaluation::Evaluator;

pub struct NushellMcpServer {
    tool_router: ToolRouter<Self>,
    evaluator: Evaluator,
}

#[tool_router]
impl NushellMcpServer {
    pub fn new(mut engine_state: EngineState) -> Self {
        // Configure the engine state for MCP
        if let Some(config) = Arc::get_mut(&mut engine_state.config) {
            config.use_ansi_coloring = UseAnsiColoring::False;
            config.color_config.clear();
        }
        NushellMcpServer {
            tool_router: Self::tool_router(),
            evaluator: Evaluator::new(engine_state),
        }
    }

    #[tool(description = r#"List available Nushell native commands.
By default all available commands will be returned. To find a specific command by searching command names, descriptions and search terms, use the find parameter."#)]
    async fn list_commands(
        &self,
        ctx: RequestContext<RoleServer>,
        Parameters(ListCommandsRequest { find }): Parameters<ListCommandsRequest>,
    ) -> Result<String, McpError> {
        let cmd = if let Some(f) = find {
            format!("help commands --find {f}")
        } else {
            "help commands".to_string()
        };

        self.evaluator.eval_async(&cmd, ctx.ct).await
    }

    #[tool(
        description = "Get help for a specific Nushell command. This will only work on commands that are native to nushell. To find out if a command is native to nushell you can use the find_command tool."
    )]
    async fn command_help(
        &self,
        ctx: RequestContext<RoleServer>,
        Parameters(CommandNameRequest { name }): Parameters<CommandNameRequest>,
    ) -> Result<String, McpError> {
        let cmd = format!("help {name}");
        self.evaluator.eval_async(&cmd, ctx.ct).await
    }

    #[doc = include_str!("evaluate_tool.md")]
    #[tool]
    async fn evaluate(
        &self,
        ctx: RequestContext<RoleServer>,
        Parameters(NuSourceRequest { input }): Parameters<NuSourceRequest>,
    ) -> Result<String, McpError> {
        self.evaluator.eval_async(&input, ctx.ct).await
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
struct ListCommandsRequest {
    #[schemars(description = "string to find in command names, descriptions, and search term")]
    find: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
struct CommandNameRequest {
    #[schemars(description = "The name of the command")]
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
            instructions: Some(include_str!("instructions.md").to_string()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}
