use std::process::Child;
use std::sync::Mutex;

use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::{tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::worker::WorkerClient;

pub struct NushellMcpServer {
    tool_router: ToolRouter<Self>,
    worker_client: Mutex<WorkerClient>,
    #[allow(dead_code)]
    worker_process: Child,
}

#[tool_router]
impl NushellMcpServer {
    pub fn new() -> Self {
        // Spawn the worker process
        let (worker_process, socket_path) =
            crate::worker::spawn_worker().expect("Failed to spawn MCP worker");

        // Connect to the worker
        let worker_client =
            WorkerClient::connect(&socket_path).expect("Failed to connect to MCP worker");

        NushellMcpServer {
            tool_router: Self::tool_router(),
            worker_client: Mutex::new(worker_client),
            worker_process,
        }
    }

    fn eval(&self, source: &str) -> Result<String, McpError> {
        let mut client = self.worker_client.lock().expect("worker lock poisoned");
        client
            .eval(source)
            .map_err(|e| McpError::internal_error(e, None))
    }

    #[tool(description = r#"List available Nushell native commands.
By default all available commands will be returned. To find a specific command by searching command names, descriptions and search terms, use the find parameter."#)]
    async fn list_commands(
        &self,
        Parameters(ListCommandsRequest { find }): Parameters<ListCommandsRequest>,
    ) -> Result<String, McpError> {
        let cmd = if let Some(f) = find {
            format!("help commands --find {f}")
        } else {
            "help commands".to_string()
        };

        self.eval(&cmd)
    }

    #[tool(
        description = "Get help for a specific Nushell command. This will only work on commands that are native to nushell. To find out if a command is native to nushell you can use the find_command tool."
    )]
    async fn command_help(
        &self,
        Parameters(CommandNameRequest { name }): Parameters<CommandNameRequest>,
    ) -> Result<String, McpError> {
        let cmd = format!("help {name}");
        self.eval(&cmd)
    }

    #[doc = include_str!("evaluate_tool.md")]
    #[tool]
    async fn evaluate(
        &self,
        Parameters(NuSourceRequest { input }): Parameters<NuSourceRequest>,
    ) -> Result<String, McpError> {
        self.eval(&input)
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
