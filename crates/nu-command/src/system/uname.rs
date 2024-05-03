use nu_protocol::record;
use nu_protocol::Value;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Type,
};

#[derive(Clone)]
pub struct UName;

impl Command for UName {
    fn name(&self) -> &str {
        "uname"
    }

    fn signature(&self) -> Signature {
        Signature::build("uname")
            .input_output_types(vec![(Type::Nothing, Type::table())])
            .category(Category::System)
    }

    fn usage(&self) -> &str {
        "Print certain system information using uutils/coreutils uname."
    }

    fn search_terms(&self) -> Vec<&str> {
        // add other terms?
        vec!["system", "coreutils"]
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let span = call.head;
        // Simulate `uname -all` is called every time
        let opts = uu_uname::Options {
            all: true,
            kernel_name: false,
            nodename: false,
            kernel_release: false,
            kernel_version: false,
            machine: false,
            processor: false,
            hardware_platform: false,
            os: false,
        };
        let output = uu_uname::UNameOutput::new(&opts).map_err(|e| ShellError::GenericError {
            error: format!("{}", e),
            msg: format!("{}", e),
            span: None,
            help: None,
            inner: Vec::new(),
        })?;
        let outputs = [
            output.kernel_name,
            output.nodename,
            output.kernel_release,
            output.kernel_version,
            output.machine,
            output.os,
        ];
        let outputs = outputs
            .iter()
            .map(|name| {
                Ok(name
                    .as_ref()
                    .ok_or("unknown")
                    .map_err(|_| ShellError::NotFound { span })?
                    .to_string())
            })
            .collect::<Result<Vec<String>, ShellError>>()?;
        Ok(PipelineData::Value(
            Value::record(
                record! {
                    "kernel-name" => Value::string(outputs[0].clone(), span),
                    "nodename" => Value::string(outputs[1].clone(), span),
                    "kernel-release" => Value::string(outputs[2].clone(), span),
                    "kernel-version" => Value::string(outputs[3].clone(), span),
                    "machine" => Value::string(outputs[4].clone(), span),
                    "operating-system" => Value::string(outputs[5].clone(), span),
                },
                span,
            ),
            None,
        ))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Print all information",
            example: "uname",
            result: None,
        }]
    }
}
