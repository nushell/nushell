use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::IntoPipelineData;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, TcpListener};

use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "port"
    }

    fn signature(&self) -> Signature {
        Signature::build("port")
            .input_output_types(vec![(Type::Nothing, Type::Int)])
            .optional(
                "start",
                SyntaxShape::Int,
                "The start port to scan (inclusive)",
            )
            .optional("end", SyntaxShape::Int, "The end port to scan (inclusive)")
            .category(Category::Network)
    }

    fn usage(&self) -> &str {
        "Get a free port from system."
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
                result: Some(Value::test_int(3121)),
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
            return Err(ShellError::InvalidRange {
                left_flank: start_port.to_string(),
                right_flank: end_port.to_string(),
                span: call.head,
            });
        }

        // try given port one by one.
        match (start_port..=end_port)
            .map(|port| SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, port as u16)))
            .find_map(|addr| TcpListener::bind(addr).ok())
        {
            Some(listener) => listener,
            None => {
                return Err(ShellError::IOError {
                    msg: "Every port has been tried, but no valid one was found".to_string(),
                })
            }
        }
    };

    let free_port = listener.local_addr()?.port();
    Ok(Value::int(free_port as i64, call.head).into_pipeline_data())
}
