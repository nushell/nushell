use crate::object::{TaggedDictBuilder, Value};
use crate::prelude::*;
use itertools::join;
use sysinfo::ProcessExt;

crate fn process_dict(proc: &sysinfo::Process, span: impl Into<Span>) -> Tagged<Value> {
    let mut dict = TaggedDictBuilder::new(span);

    let cmd = proc.cmd();

    let cmd_value = if cmd.len() == 0 {
        Value::nothing()
    } else {
        Value::string(join(cmd, ""))
    };

    dict.insert("pid", Value::int(proc.pid() as i64));
    dict.insert("status", Value::string(proc.status().to_string()));
    dict.insert("cpu", Value::float(proc.cpu_usage() as f64));
    //dict.insert("name", Value::string(proc.name()));
    match cmd_value {
        Value::Primitive(Primitive::Nothing) => {
            dict.insert("name", Value::string(proc.name()));
        }
        _ => dict.insert("name", cmd_value),
    }

    dict.into_tagged_value()
}
