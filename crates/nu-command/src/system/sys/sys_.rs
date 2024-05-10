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

    fn extra_usage(&self) -> &str {
        "Note that this command may take a noticeable amount of time to run. To reduce the time taken, you can use the various `sys` sub commands to get the subset of information you are interested in."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        nu_protocol::report_error_new(
            engine_state,
            &ShellError::GenericError {
                error: "Deprecated command".into(),
                msg: "the `sys` command is deprecated, please use the new subcommands (`sys host`, `sys mem`, etc.)."
                    .into(),
                span: Some(call.head),
                help: None,
                inner: vec![],
            },
        );

        let head = call.head;

        let mut host = super::host(head);
        host.push("sessions", super::users(head));
        let record = record! {
            "host" => Value::record(host, head),
            "cpu" => super::cpu(head),
            "disks" => super::disks(head),
            "mem" => super::mem(head),
            "temp" => super::temp(head),
            "net" => super::net(head),
        };
        Ok(Value::record(record, head).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Show info about the system",
            example: "sys",
            result: None,
        }]
    }
}
