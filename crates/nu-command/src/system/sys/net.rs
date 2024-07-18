use super::trim_cstyle_null;
use nu_engine::command_prelude::*;
use sysinfo::Networks;

#[derive(Clone)]
pub struct SysNet;

impl Command for SysNet {
    fn name(&self) -> &str {
        "sys net"
    }

    fn signature(&self) -> Signature {
        Signature::build("sys net")
            .filter()
            .category(Category::System)
            .input_output_types(vec![(Type::Nothing, Type::table())])
    }

    fn usage(&self) -> &str {
        "View information about the system network interfaces."
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Ok(net(call.head).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Show info about the system network",
            example: "sys net",
            result: None,
        }]
    }
}

fn net(span: Span) -> Value {
    let networks = Networks::new_with_refreshed_list()
        .iter()
        .map(|(iface, data)| {
            let record = record! {
                "name" => Value::string(trim_cstyle_null(iface), span),
                "sent" => Value::filesize(data.total_transmitted() as i64, span),
                "recv" => Value::filesize(data.total_received() as i64, span),
            };

            Value::record(record, span)
        })
        .collect();

    Value::list(networks, span)
}
