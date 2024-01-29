use nu_engine::CallExt;
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
            .switch(
                "all",
                "print all information, except omit -p and -i if unknown",
                Some('a'),
            )
            .switch("kernel-name", "print the kernel name", Some('s'))
            .switch("nodename", "print the network node hostname", Some('n'))
            .switch("kernel-release", "print the kernel release", Some('r'))
            .switch("kernel-version", "print the kernel version", Some('v'))
            .switch("machine", "print the machine hardware name", Some('m'))
            .switch(
                "processor",
                "print the processor type (non-portable)",
                Some('p'),
            )
            .switch(
                "hardware-platform",
                "print the hardware platform (non-portable)",
                Some('i'),
            )
            .switch("operating-system", "print the operating system", Some('o'))
            .category(Category::System)
    }

    fn usage(&self) -> &str {
        "Print certain system information.  With no OPTION, same as -s."
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
        let all = call.has_flag(engine_state, stack, "all")?;
        let kernel_name = call.has_flag(engine_state, stack, "kernel-name")?;
        let nodename = call.has_flag(engine_state, stack, "nodename")?;
        let kernel_release = call.has_flag(engine_state, stack, "kernel-release")?;
        let kernel_version = call.has_flag(engine_state, stack, "kernel-version")?;
        let machine = call.has_flag(engine_state, stack, "machine")?;
        let processor = call.has_flag(engine_state, stack, "processor")?;
        let hardware_platform = call.has_flag(engine_state, stack, "hardware-platform")?;
        let os = call.has_flag(engine_state, stack, "operating-system")?;
        let opts = uu_uname::Options {
            all,
            kernel_name,
            nodename,
            kernel_release,
            kernel_version,
            machine,
            processor,
            hardware_platform,
            os,
        };
        if let Err(error) = uu_uname::uu_uname(&opts) {
            return Err(ShellError::GenericError {
                error: format!("{}", error),
                msg: format!("{}", error),
                span: None,
                help: None,
                inner: Vec::new(),
            });
        }
        Ok(PipelineData::empty())
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
