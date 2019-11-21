use crate::data::{TaggedDictBuilder, Value};
use crate::prelude::*;
use itertools::join;
use sysinfo::ProcessExt;

pub(crate) fn process_dict(proc: &sysinfo::Process, tag: impl Into<Tag>) -> Value {
    let mut dict = TaggedDictBuilder::new(tag);

    let cmd = proc.cmd();

    let cmd_value = if cmd.len() == 0 {
        UntaggedValue::nothing()
    } else {
        UntaggedValue::string(join(cmd, ""))
    };

    dict.insert("pid", UntaggedValue::int(proc.pid() as i64));
    dict.insert("status", UntaggedValue::string(proc.status().to_string()));
    dict.insert("cpu", UntaggedValue::number(proc.cpu_usage()));

    match cmd_value {
        UntaggedValue::Primitive(Primitive::Nothing) => {
            dict.insert("name", UntaggedValue::string(proc.name()));
        }
        _ => dict.insert("name", cmd_value),
    }

    dict.into_value()
}
