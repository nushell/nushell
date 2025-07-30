use nu_protocol::{ShellError, engine::EngineState};
use rmcp::{ServiceExt, transport::stdio};
use server::NushellMcpServer;
use tokio::runtime::Runtime;
use tracing_subscriber::EnvFilter;

mod server;

pub fn initialize_mcp_server(engine_state: EngineState) -> Result<(), ShellError> {
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

    runtime.spawn(async {
        if let Err(e) = run_server(engine_state).await {
            tracing::error!("Error running MCP server: {:?}", e);
        }
    });
    Ok(())
}

async fn run_server(engine_state: EngineState) -> Result<(), Box<dyn std::error::Error>> {
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
