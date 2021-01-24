use nu_errors::ShellError;
use nu_protocol::{TaggedDictBuilder, UntaggedValue, Value};
use nu_source::Tag;
use sysinfo::{ProcessExt, System, SystemExt};

#[derive(Default)]
pub struct Ps;

impl Ps {
    pub fn new() -> Ps {
        Ps
    }
}

pub async fn ps(tag: Tag, long: bool) -> Result<Vec<Value>, ShellError> {
    let mut sys = System::new_all();
    sys.refresh_all();
    let duration = std::time::Duration::from_millis(500);
    std::thread::sleep(duration);
    sys.refresh_all();

    let mut output = vec![];

    let result = sys.get_processes();

    for (pid, process) in result.iter() {
        let mut dict = TaggedDictBuilder::new(&tag);
        dict.insert_untagged("pid", UntaggedValue::int(*pid));
        dict.insert_untagged("name", UntaggedValue::string(process.name()));
        dict.insert_untagged(
            "status",
            UntaggedValue::string(format!("{:?}", process.status())),
        );
        dict.insert_untagged(
            "cpu",
            UntaggedValue::decimal_from_float(process.cpu_usage() as f64, tag.span),
        );
        dict.insert_untagged("mem", UntaggedValue::filesize(process.memory() * 1000));
        dict.insert_untagged(
            "virtual",
            UntaggedValue::filesize(process.virtual_memory() * 1000),
        );

        if long {
            if let Some(parent) = process.parent() {
                dict.insert_untagged("parent", UntaggedValue::int(parent));
            }
            dict.insert_untagged("exe", UntaggedValue::filepath(process.exe()));
            dict.insert_untagged("command", UntaggedValue::string(process.cmd().join(" ")));
        }

        output.push(dict.into_value());
    }

    Ok(output)
}
