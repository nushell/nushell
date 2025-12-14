use nu_protocol::{ShellError, engine::EngineState, engine::StateWorkingSet, format_cli_error};
use rmcp::{ServiceExt, transport::stdio};
use server::NushellMcpServer;
use tokio::runtime::Runtime;
use tracing_subscriber::EnvFilter;

use rmcp::ErrorData as McpError;

mod evaluation;
mod history;
mod server;
pub mod worker;

pub fn initialize_mcp_server() -> Result<(), ShellError> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::DEBUG.into()))
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    tracing::info!("Starting MCP server");
    let runtime = Runtime::new().map_err(|e| ShellError::GenericError {
        error: format!("Could not instantiate tokio: {e}"),
        msg: "".into(),
        span: None,
        help: None,
        inner: vec![],
    })?;

    runtime.block_on(async {
        if let Err(e) = run_server().await {
            tracing::error!("Error running MCP server: {:?}", e);
        }
    });
    Ok(())
}

/// Run the MCP worker process that handles nushell evaluation.
/// This is called when `nu --mcp-worker <socket-path>` is invoked.
pub fn run_mcp_worker(
    socket_path: std::path::PathBuf,
    engine_state: EngineState,
) -> Result<(), ShellError> {
    worker::run_worker(socket_path, engine_state).map_err(|e| ShellError::GenericError {
        error: format!("MCP worker error: {e}"),
        msg: "".into(),
        span: None,
        help: None,
        inner: vec![],
    })
}

async fn run_server() -> Result<(), Box<dyn std::error::Error>> {
    NushellMcpServer::new()
        .serve(stdio())
        .await
        .inspect_err(|e| {
            tracing::error!("serving error: {:?}", e);
        })?
        .waiting()
        .await?;
    Ok(())
}

pub(crate) fn shell_error_to_mcp_error(
    error: nu_protocol::ShellError,
    engine_state: &EngineState,
) -> McpError {
    // Use Nushell's rich error formatting to provide detailed, helpful error messages for LLMs
    let working_set = StateWorkingSet::new(engine_state);
    let formatted_error = format_cli_error(None, &working_set, &error, Some("nu::shell::error"));
    McpError::internal_error(formatted_error, None)
}
