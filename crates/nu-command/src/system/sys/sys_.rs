use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct Sys;

impl Command for Sys {
    fn name(&self) -> &str {
        "sys"
    }

    fn signature(&self) -> Signature {
        Signature::build("sys")
            .filter()
            .category(Category::System)
            .input_output_types(vec![(Type::Nothing, Type::record())])
    }

    fn usage(&self) -> &str {
        "View information about the system."
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let record = record! {
            "host" => super::host(head),
            "cpu" => super::cpu(head),
            "disks" => super::disks(head),
            "mem" => super::mem(head),
            "temp" => super::temp(head),
            "net" => super::net(head),
        };
        Ok(Value::record(record, head).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Show info about the system",
                example: "sys",
                result: None,
            },
            Example {
                description: "Show the os system name with get",
                example: "(sys).host | get name",
                result: None,
            },
            Example {
                description: "Show the os system name",
                example: "(sys).host.name",
                result: None,
            },
        ]
    }
}
