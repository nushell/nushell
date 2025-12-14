//! Worker process for MCP evaluation.
//!
//! The MCP server uses stdio for JSON-RPC communication, which conflicts with
//! external commands that want to inherit stdin/stdout. To solve this, we spawn
//! a separate worker process that handles the actual nushell evaluation.
//!
//! Architecture:
//! - Parent process: Handles MCP stdio transport, forwards eval requests to worker
//! - Worker process: Evaluates nushell code with its own stdin/stdout

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

use nu_protocol::engine::EngineState;
use serde::{Deserialize, Serialize};

use crate::evaluation::Evaluator;

/// Request sent from parent to worker
#[derive(Debug, Serialize, Deserialize)]
pub struct EvalRequest {
    pub source: String,
}

/// Response sent from worker to parent
#[derive(Debug, Serialize, Deserialize)]
pub struct EvalResponse {
    pub result: Result<String, String>,
}

/// Get the socket path for worker communication
pub fn socket_path() -> PathBuf {
    let pid = std::process::id();
    std::env::temp_dir().join(format!("nu-mcp-worker-{pid}.sock"))
}

/// Spawns a worker process and returns its handle
pub fn spawn_worker() -> std::io::Result<(Child, PathBuf)> {
    let sock_path = socket_path();

    // Remove old socket if exists
    let _ = std::fs::remove_file(&sock_path);

    let current_exe = std::env::current_exe()?;

    let child = Command::new(current_exe)
        .arg("--no-config-file") // Skip config files to avoid interference with external command handling
        .arg("--mcp-worker")
        .arg(&sock_path)
        .stdin(Stdio::null())
        .stdout(Stdio::null()) // Isolated from MCP's JSON-RPC channel
        .stderr(Stdio::null()) // Isolated from MCP's JSON-RPC channel
        .spawn()?;

    Ok((child, sock_path))
}

/// Client that communicates with the worker process
pub struct WorkerClient {
    stream: UnixStream,
}

impl WorkerClient {
    /// Connect to the worker, retrying until it's ready
    pub fn connect(socket_path: &std::path::Path) -> std::io::Result<Self> {
        // Wait for worker to be ready
        for _ in 0..50 {
            match UnixStream::connect(socket_path) {
                Ok(stream) => return Ok(Self { stream }),
                Err(_) => std::thread::sleep(std::time::Duration::from_millis(100)),
            }
        }
        UnixStream::connect(socket_path).map(|stream| Self { stream })
    }

    /// Send an evaluation request and get the response
    pub fn eval(&mut self, source: &str) -> Result<String, String> {
        let request = EvalRequest {
            source: source.to_string(),
        };

        let request_json = serde_json::to_string(&request).map_err(|e| e.to_string())?;

        writeln!(self.stream, "{}", request_json).map_err(|e| e.to_string())?;
        self.stream.flush().map_err(|e| e.to_string())?;

        let mut reader = BufReader::new(&self.stream);
        let mut response_line = String::new();
        reader
            .read_line(&mut response_line)
            .map_err(|e| e.to_string())?;

        let response: EvalResponse =
            serde_json::from_str(&response_line).map_err(|e| e.to_string())?;

        response.result
    }
}

/// Run the worker process - listens on a unix socket and evaluates requests
pub fn run_worker(socket_path: PathBuf, engine_state: EngineState) -> Result<(), std::io::Error> {
    // Remove old socket if exists
    let _ = std::fs::remove_file(&socket_path);

    let listener = UnixListener::bind(&socket_path)?;
    let evaluator = Evaluator::new(engine_state);

    eprintln!("MCP worker listening on {:?}", socket_path);

    // Accept one connection (from the parent MCP process)
    let (stream, _) = listener.accept()?;
    let mut reader = BufReader::new(&stream);
    let mut writer = stream.try_clone()?;

    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => break, // EOF - parent closed connection
            Ok(_) => {}
            Err(e) => {
                eprintln!("Worker read error: {e}");
                break;
            }
        }

        let request: EvalRequest = match serde_json::from_str(&line) {
            Ok(req) => req,
            Err(e) => {
                eprintln!("Worker parse error: {e}");
                continue;
            }
        };

        let result = evaluator
            .eval(&request.source)
            .map_err(|e| e.message.to_string());

        let response = EvalResponse { result };
        let response_json = serde_json::to_string(&response).unwrap_or_else(|e| {
            serde_json::to_string(&EvalResponse {
                result: Err(e.to_string()),
            })
            .unwrap()
        });

        if writeln!(writer, "{}", response_json).is_err() {
            break;
        }
        if writer.flush().is_err() {
            break;
        }
    }

    // Cleanup
    let _ = std::fs::remove_file(&socket_path);
    Ok(())
}
