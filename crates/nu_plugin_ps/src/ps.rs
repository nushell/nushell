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

    let mut output = vec![];

    let result: Vec<_> = sys.get_processes().iter().map(|x| *x.0).collect();

    for pid in result.into_iter() {
        sys.refresh_process(pid);
        if let Some(result) = sys.get_process(pid) {
            let mut dict = TaggedDictBuilder::new(&tag);
            dict.insert_untagged("pid", UntaggedValue::int(pid));
            dict.insert_untagged("name", UntaggedValue::string(result.name()));
            dict.insert_untagged(
                "status",
                UntaggedValue::string(format!("{:?}", result.status())),
            );
            dict.insert_untagged(
                "cpu",
                UntaggedValue::decimal_from_float(result.cpu_usage() as f64, tag.span),
            );
            dict.insert_untagged("mem", UntaggedValue::filesize(result.memory() * 1000));
            dict.insert_untagged(
                "virtual",
                UntaggedValue::filesize(result.virtual_memory() * 1000),
            );

            if long {
                if let Some(parent) = result.parent() {
                    dict.insert_untagged("parent", UntaggedValue::int(parent));
                } else {
                    dict.insert_untagged("parent", UntaggedValue::nothing());
                }
                dict.insert_untagged("exe", UntaggedValue::filepath(result.exe()));
                dict.insert_untagged("command", UntaggedValue::string(result.cmd().join(" ")));
            }

            output.push(dict.into_value());
        }
    }

    Ok(output)
}
