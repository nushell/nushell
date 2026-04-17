use crate::evaluation::Evaluator;
use nu_protocol::{UseAnsiColoring, engine::EngineState};
use rmcp::{
    RoleServer, ServerHandler,
    handler::server::{tool::ToolRouter, wrapper::Parameters},
    model::{Implementation, ServerCapabilities, ServerInfo},
    service::RequestContext,
    tool, tool_handler, tool_router,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

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
        _ctx: RequestContext<RoleServer>,
        Parameters(ListCommandsRequest { find }): Parameters<ListCommandsRequest>,
    ) -> Result<String, String> {
        self.evaluator.list_available_commands(find).await
    }

    #[tool(
        description = "Get help for a specific Nushell command. This will only work on commands that are native to nushell. To find out if a command is native to nushell you can use the list_commands tool."
    )]
    async fn command_help(
        &self,
        _ctx: RequestContext<RoleServer>,
        Parameters(CommandNameRequest { name }): Parameters<CommandNameRequest>,
    ) -> Result<String, String> {
        self.evaluator.command_help(&name).await
    }

    #[doc = include_str!("evaluate_tool.md")]
    #[tool]
    async fn evaluate(
        &self,
        ctx: RequestContext<RoleServer>,
        Parameters(NuSourceRequest {
            input,
            timeout_secs,
        }): Parameters<NuSourceRequest>,
    ) -> Result<String, String> {
        let timeout_override = timeout_secs
            .filter(|secs| secs.is_finite() && *secs > 0.0)
            .map(Duration::from_secs_f64);
        self.evaluator
            .eval_async(&input, ctx.ct, timeout_override)
            .await
            .map_err(|err| err.message.to_string())
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
    /// Seconds before this call is promoted to a background job. See the tool
    /// description for details and precedence rules.
    #[schemars(
        description = "Seconds before this call is promoted to a background job (default 120). Set higher for long builds/tests."
    )]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    timeout_secs: Option<f64>,
}

#[tool_handler]
impl ServerHandler for NushellMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(
                Implementation::new("nushell-mcp-server", env!("CARGO_PKG_VERSION"))
                    .with_title("Nushell MCP Server")
                    .with_website_url("https://www.nushell.sh"),
            )
            .with_instructions(include_str!("instructions.md"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::channel::mpsc;
    use nu_cmd_lang::create_default_context;
    use rmcp::model::RequestId;
    use rmcp::service::{RxJsonRpcMessage, TxJsonRpcMessage, serve_directly};
    use serde_json::Value as JsonValue;

    fn make_request_context(request_id: i64) -> RequestContext<RoleServer> {
        let engine_state = create_default_context();
        let server = NushellMcpServer::new(engine_state);

        let (tx, _rx_sink) = mpsc::unbounded::<TxJsonRpcMessage<RoleServer>>();
        let (_tx_stream, rx_stream) = mpsc::unbounded::<RxJsonRpcMessage<RoleServer>>();
        let transport = (tx, rx_stream);

        let running = serve_directly(server, transport, None);
        let peer = running.peer().clone();
        drop(running);

        RequestContext::new(RequestId::Number(request_id), peer)
    }

    #[test]
    fn server_info_serializes_expected_mcp_metadata() {
        let engine_state = create_default_context();
        let server = NushellMcpServer::new(engine_state);
        let info = server.get_info();
        let json = serde_json::to_value(&info).expect("ServerInfo should serialize to JSON");

        let capabilities = json
            .get("capabilities")
            .expect("ServerInfo JSON should include capabilities");
        let tools = capabilities
            .get("tools")
            .and_then(JsonValue::as_object)
            .expect("Server capabilities should include tools");
        assert!(
            tools.is_empty(),
            "tools capability should be present as an empty object"
        );

        let instructions = json
            .get("instructions")
            .expect("ServerInfo JSON should include instructions")
            .as_str()
            .expect("instructions should be a string");
        assert!(
            instructions.contains("Nushell native commands"),
            "instructions should document command discovery"
        );

        let server_info = json
            .get("serverInfo")
            .or_else(|| json.get("server_info"))
            .expect("ServerInfo JSON should include serverInfo");
        assert_eq!(
            server_info.get("name").and_then(JsonValue::as_str),
            Some("nushell-mcp-server")
        );
        assert_eq!(
            server_info.get("title").and_then(JsonValue::as_str),
            Some("Nushell MCP Server")
        );
        assert_eq!(
            server_info.get("version").and_then(JsonValue::as_str),
            Some(env!("CARGO_PKG_VERSION"))
        );
        assert_eq!(
            server_info
                .get("websiteUrl")
                .or_else(|| server_info.get("website_url"))
                .and_then(JsonValue::as_str),
            Some("https://www.nushell.sh")
        );
    }

    #[test]
    fn tool_router_exposes_expected_mcp_tools() {
        let router = NushellMcpServer::tool_router();
        let tool_names: Vec<_> = router
            .list_all()
            .iter()
            .map(|tool| tool.name.clone())
            .collect();
        assert_eq!(
            tool_names,
            vec![
                "command_help".to_string(),
                "evaluate".to_string(),
                "list_commands".to_string()
            ]
        );

        let list_commands_tool = router
            .get("list_commands")
            .expect("list_commands tool should be registered");
        assert!(
            list_commands_tool
                .description
                .as_deref()
                .unwrap_or("")
                .contains("List available Nushell native commands"),
            "list_commands tool description should mention native command discovery"
        );
    }

    fn create_mcp_server() -> NushellMcpServer {
        let engine_state = create_default_context();
        NushellMcpServer::new(engine_state)
    }

    #[tokio::test]
    async fn list_commands_tool_returns_non_empty_help() {
        let server = create_mcp_server();
        let ctx = make_request_context(0);

        let result = server
            .list_commands(
                ctx,
                Parameters(ListCommandsRequest {
                    find: Some("version".to_string()),
                }),
            )
            .await
            .expect("list_commands should succeed");

        assert!(
            result.len() > 20,
            "list_commands output should not be empty"
        );
        assert!(
            result.to_lowercase().contains("version"),
            "list_commands output should mention version"
        );
    }

    #[tokio::test]
    async fn evaluate_tool_computes_basic_expression() {
        let server = create_mcp_server();
        let ctx = make_request_context(1);

        let result = server
            .evaluate(
                ctx,
                Parameters(NuSourceRequest {
                    input: "5 + 2".to_string(),
                    timeout_secs: None,
                }),
            )
            .await
            .expect("evaluate should succeed");

        assert!(
            result.contains("output"),
            "evaluate output should include output metadata"
        );
        assert!(
            result.contains('7'),
            "evaluate output should include the computed result"
        );
    }

    #[tokio::test]
    async fn evaluate_tool_history_index_increments() {
        let engine_state = nu_cmd_lang::create_default_context();
        let server = NushellMcpServer::new(engine_state);

        let result1 = server
            .evaluate(
                make_request_context(3),
                Parameters(NuSourceRequest {
                    input: "1".to_string(),
                    timeout_secs: None,
                }),
            )
            .await
            .expect("first evaluate should succeed");
        assert!(
            result1.contains("history_index:0") || result1.contains("history_index: 0"),
            "first evaluation should have history_index 0"
        );

        let result2 = server
            .evaluate(
                make_request_context(4),
                Parameters(NuSourceRequest {
                    input: "2".to_string(),
                    timeout_secs: None,
                }),
            )
            .await
            .expect("second evaluate should succeed");
        assert!(
            result2.contains("history_index:1") || result2.contains("history_index: 1"),
            "second evaluation should have history_index 1"
        );
    }

    #[tokio::test]
    async fn command_help_tool_returns_help_for_version_command() {
        let server = create_mcp_server();
        let ctx = make_request_context(2);

        let result = server
            .command_help(
                ctx,
                Parameters(CommandNameRequest {
                    name: "version".to_string(),
                }),
            )
            .await
            .expect("command_help should succeed");

        assert!(
            result.to_lowercase().contains("version"),
            "command_help output should mention the command name"
        );
        assert!(
            result.to_lowercase().contains("usage") || result.contains("Usage"),
            "command_help output should include usage information"
        );
    }
}
