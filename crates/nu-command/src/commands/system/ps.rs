use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Signature, TaggedDictBuilder, UntaggedValue};
use sysinfo::{ProcessExt, System, SystemExt};

pub struct Command;

impl WholeStreamCommand for Command {
    fn name(&self) -> &str {
        "ps"
    }

    fn signature(&self) -> Signature {
        Signature::build("ps")
            .desc("View information about system processes.")
            .switch(
                "long",
                "list all available columns for each entry",
                Some('l'),
            )
            .filter()
    }

    fn usage(&self) -> &str {
        "View information about system processes."
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        run_ps(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "List the system processes",
            example: "ps",
            result: None,
        }]
    }
}

fn run_ps(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let long = args.has_flag("long");
    let mut sys = System::new_all();
    sys.refresh_all();

    let mut output = vec![];

    let result: Vec<_> = sys.processes().iter().map(|x| *x.0).collect();

    for pid in result {
        if let Some(result) = sys.process(pid) {
            let mut dict = TaggedDictBuilder::new(args.name_tag());
            dict.insert_untagged("pid", UntaggedValue::int(pid as i64));
            dict.insert_untagged("name", UntaggedValue::string(result.name()));
            dict.insert_untagged(
                "status",
                UntaggedValue::string(format!("{:?}", result.status())),
            );
            dict.insert_untagged(
                "cpu",
                UntaggedValue::decimal_from_float(result.cpu_usage() as f64, args.name_tag().span),
            );
            dict.insert_untagged("mem", UntaggedValue::filesize(result.memory() * 1000));
            dict.insert_untagged(
                "virtual",
                UntaggedValue::filesize(result.virtual_memory() * 1000),
            );

            if long {
                if let Some(parent) = result.parent() {
                    dict.insert_untagged("parent", UntaggedValue::int(parent as i64));
                } else {
                    dict.insert_untagged("parent", UntaggedValue::nothing());
                }
                dict.insert_untagged("exe", UntaggedValue::filepath(result.exe()));
                dict.insert_untagged("command", UntaggedValue::string(result.cmd().join(" ")));
            }

            output.push(dict.into_value());
        }
    }

    Ok(output.into_iter().into_output_stream())
}
