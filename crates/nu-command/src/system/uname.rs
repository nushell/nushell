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
        // Linux netname 5.15.0-91-generic #101~20.04.1-Ubuntu SMP Thu Nov 16 14:22:28 UTC 2023 x86_64 GNU/Linux

        // Value::record(
        //     record! {
        //         "name" => Value::string(get_os_name(), span),
        //         "arch" => Value::string(get_os_arch(), span),
        //         "family" => Value::string(get_os_family(), span),
        //         "kernel_version" => Value::string(ver, span),
        //     },
        //     span,
        // )
        let span = call.head;
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
        let output = uu_uname::uu_uname(&opts).map_err(|e| ShellError::GenericError {
            error: format!("{}", e),
            msg: format!("{}", e),
            span: None,
            help: None,
            inner: Vec::new(),
        })?;
        let output = output.trim_end();
        // Output for now always follows the string as followed
        // -s -n -r -v -m -o, omitting -p and -i always (uutils)
        // Linux netname 5.15.0-91-generic #101~20.04.1-Ubuntu SMP Thu Nov 16 14:22:28 UTC 2023 x86_64 GNU/Linux
        let output_vec = output.split(' ').collect::<Vec<&str>>();

        // TODO
        // Check that the length equals how many times i want to divide
        // That way I can ensure the values are correct.
        // Can anyone check the format of kernel-version?
        let mut value = Value::record(
            record! {
                "kernel-name" => Value::string(output_vec[0], span),
                "nodename" => Value::string(output_vec[1], span),
                "kernel-release" => Value::string(output_vec[2], span),
                "kernel-version" => Value::string(output_vec[3..11].join(" "), span),
                "machine" => Value::string(output_vec[11], span),
                "operating-system" => Value::string(output_vec.last().expect("fix").clone(), span),
            },
            span,
        );
        // println!("{:?}", record);
        // let z = record.int;

        // Ok(PipelineData::Value(
        //     Value::Record {
        //         val: record,
        //         internal_span: call.head,
        //     },
        //     None,
        // ))
        //
        Ok(PipelineData::Value(value, None))
        // Ok(Value::record(record, call.head))

        // Ok(PipelineData::empty())
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
