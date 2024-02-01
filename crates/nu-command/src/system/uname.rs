use nu_engine::CallExt;
use nu_protocol::record;
use nu_protocol::Record;
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
            .input_output_types(vec![(Type::Nothing, Type::Table(vec![]))])
            .category(Category::System)
    }

    fn usage(&self) -> &str {
        "Print certain system information"
    }

    fn search_terms(&self) -> Vec<&str> {
        // add other terms?
        vec!["system"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
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
        let raw_output = uu_uname::uu_uname(&opts).map_err(|e| ShellError::GenericError {
            error: format!("{}", e),
            msg: format!("{}", e),
            span: None,
            help: None,
            inner: Vec::new(),
        })?;
        let output = raw_output.trim_end().split(' ').collect::<Vec<&str>>();
        // Output for now always follows the string as followed
        // -s -n -r -v -m -o, omitting -p and -i always (uutils)
        // Linux netname 5.15.0-91-generic #101~20.04.1-Ubuntu SMP Thu Nov 16 14:22:28 UTC 2023 x86_64 GNU/Linux
        // Is this guaranteed? Ask around if `uname -all` could have any other format than the one
        // above.
        match output.as_slice() {
            [kernel_name, nodename, kernel_release, kernel_version @ .., machine, os] => {
                Ok(PipelineData::Value(
                    Value::record(
                        record! {
                            "kernel-name" => Value::string(*kernel_name, span),
                            "nodename" => Value::string(*nodename, span),
                            "kernel-release" => Value::string(*kernel_release, span),
                            "kernel-version" => Value::string(kernel_version.join(" "), span),
                            "machine" => Value::string(*machine, span),
                            "operating-system" => Value::string(*os, span),
                        },
                        span,
                    ),
                    None,
                ))
            }
            _ => Err(ShellError::GenericError {
                error: format!("Could not parse {} correctly", output.join(" ")),
                msg: "Unexpected uname output".to_string(),
                span: Some(span),
                help: None,
                inner: Vec::new(),
            }),
            // or maybe jsut return the unformatted string?
            // println!("{}",raw_output.trim_end());
            // ask to core team
            // Ok(PipelineData::empty())
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Print the kernel release",
                example: "uname -r",
                result: None,
            },
            Example {
                description: "Print all information",
                example: "uname -a",
                result: None,
            },
            Example {
                description: "Print the operating system",
                example: "uname -o",
                result: None,
            },
        ]
    }
}
