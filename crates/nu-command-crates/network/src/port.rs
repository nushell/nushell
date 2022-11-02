use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::IntoPipelineData;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, TcpListener};

use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "port"
    }

    fn signature(&self) -> Signature {
        Signature::build("port")
            .optional(
                "start",
                SyntaxShape::Int,
                "The start port to scan (inclusive)",
            )
            .optional("end", SyntaxShape::Int, "The end port to scan (inclusive)")
            .category(Category::Network)
    }

    fn usage(&self) -> &str {
        "Get a free port from system"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["network", "http"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        get_free_port(engine_state, stack, call)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "get a free port between 3121 and 4000",
                example: "port 3121 4000",
                result: Some(Value::Int {
                    val: 3121,
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "get a free port from system",
                example: "port",
                result: None,
            },
        ]
    }
}

fn get_free_port(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<PipelineData, ShellError> {
    let start_port: Option<usize> = call.opt(engine_state, stack, 0)?;
    let end_port: Option<usize> = call.opt(engine_state, stack, 1)?;

    let listener = if start_port.is_none() && end_port.is_none() {
        // get free port from system.
        TcpListener::bind("127.0.0.1:0")?
    } else {
        let start_port = start_port.unwrap_or(1024);
        let end_port = end_port.unwrap_or(65535);

        // check input range valid.
        if start_port > end_port {
            return Err(ShellError::InvalidRange(
                start_port.to_string(),
                end_port.to_string(),
                call.head,
            ));
        }

        // try given port one by one.
        let addrs: Vec<SocketAddr> = (start_port..=end_port)
            .map(|current| {
                SocketAddr::V4(SocketAddrV4::new(
                    Ipv4Addr::new(127, 0, 0, 1),
                    current as u16,
                ))
            })
            .collect();
        TcpListener::bind(addrs.as_slice())?
    };

    let free_port = listener.local_addr()?.port();
    Ok(Value::Int {
        val: free_port as i64,
        span: call.head,
    }
    .into_pipeline_data())
}
