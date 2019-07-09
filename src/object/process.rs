use crate::object::{SpannedDictBuilder, Value};
use crate::prelude::*;
use itertools::join;
use sysinfo::ProcessExt;

crate fn process_dict(proc: &sysinfo::Process, span: impl Into<Span>) -> Spanned<Value> {
    let mut dict = SpannedDictBuilder::new(span);
    dict.insert("name", Value::string(proc.name()));

    let cmd = proc.cmd();

    let cmd_value = if cmd.len() == 0 {
        Value::nothing()
    } else {
        Value::string(join(cmd, ""))
    };

    dict.insert("cmd", cmd_value);
    dict.insert("cpu", Value::float(proc.cpu_usage() as f64));
    dict.insert("pid", Value::int(proc.pid() as i64));
    dict.insert("status", Value::string(proc.status().to_string()));

    dict.into_spanned_value()
}
