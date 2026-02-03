use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use nu_protocol::{ShellError, engine::EngineState, engine::StateWorkingSet, format_cli_error};
use rmcp::{
    ServiceExt,
    transport::{
        stdio,
        streamable_http_server::{
            StreamableHttpServerConfig, StreamableHttpService,
            session::local::{LocalSessionManager, SessionConfig},
        },
    },
};
use server::NushellMcpServer;
use tokio::runtime::Runtime;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
use tracing_subscriber::EnvFilter;

use rmcp::ErrorData as McpError;

mod evaluation;
mod history;
mod server;

/// MCP transport configuration
#[derive(Debug, Clone, Default)]
pub enum McpTransport {
    /// Standard IO transport (default)
    #[default]
    Stdio,
    /// HTTP transport with SSE streaming
    Http {
        /// Port to listen on
        port: u16,
    },
}

pub fn initialize_mcp_server(
    engine_state: EngineState,
    transport: McpTransport,
) -> Result<(), ShellError> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::DEBUG.into()))
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    tracing::info!(?transport, "Starting MCP server");
    let runtime = Runtime::new().map_err(|e| ShellError::GenericError {
        error: format!("Could not instantiate tokio: {e}"),
        msg: "".into(),
        span: None,
        help: None,
        inner: vec![],
    })?;

    runtime.block_on(async {
        let result = match transport {
            McpTransport::Stdio => run_stdio_server(engine_state).await,
            McpTransport::Http { port } => run_http_server(engine_state, port).await,
        };
        if let Err(e) = result {
            tracing::error!("Error running MCP server: {:?}", e);
        }
    });
    Ok(())
}

async fn run_stdio_server(engine_state: EngineState) -> Result<(), Box<dyn std::error::Error>> {
    NushellMcpServer::new(engine_state)
        .serve(stdio())
        .await
        .inspect_err(|e| {
            tracing::error!("serving error: {:?}", e);
        })?
        .waiting()
        .await?;
    Ok(())
}

/// Session idle timeout before cleanup (30 minutes)
const SESSION_KEEP_ALIVE: Duration = Duration::from_secs(30 * 60);

/// Channel capacity for session message buffering
const SESSION_CHANNEL_CAPACITY: usize = 16;

/// SSE keep-alive ping interval
const SSE_KEEP_ALIVE: Duration = Duration::from_secs(15);

/// SSE retry interval for client reconnection
const SSE_RETRY: Duration = Duration::from_secs(3);

async fn run_http_server(
    engine_state: EngineState,
    port: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    let engine_state = Arc::new(engine_state);

    let session_manager = Arc::new(LocalSessionManager {
        sessions: RwLock::new(HashMap::new()),
        session_config: SessionConfig {
            channel_capacity: SESSION_CHANNEL_CAPACITY,
            keep_alive: Some(SESSION_KEEP_ALIVE),
        },
    });

    let service = StreamableHttpService::new(
        {
            let engine_state = engine_state.clone();
            move || Ok(NushellMcpServer::new((*engine_state).clone()))
        },
        session_manager,
        StreamableHttpServerConfig {
            sse_keep_alive: Some(SSE_KEEP_ALIVE),
            sse_retry: Some(SSE_RETRY),
            stateful_mode: true,
            cancellation_token: CancellationToken::new(),
        },
    );

    let router = Router::new().fallback_service(service);
    let addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("MCP HTTP server listening on http://{addr}");
    eprintln!("MCP HTTP server listening on http://{addr}");

    axum::serve(listener, router)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
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
