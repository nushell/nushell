use crate::object::base::Value;
use crate::object::dict::Dictionary;
use itertools::join;
use sysinfo::ProcessExt;

crate fn process_dict(proc: &sysinfo::Process) -> Dictionary {
    let mut dict = Dictionary::default();
    dict.add("name", Value::string(proc.name()));

    let cmd = proc.cmd();

    let cmd_value = if cmd.len() == 0 {
        Value::nothing()
    } else {
        Value::string(join(cmd, ""))
    };

    dict.add("cmd", cmd_value);
    dict.add("pid", Value::int(proc.pid() as i64));
    dict.add("status", Value::int(proc.status() as i64));

    dict
}
