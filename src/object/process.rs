use crate::object::base::{Primitive, ShellObject, Value};
use crate::object::desc::DataDescriptor;
use crate::object::dict::Dictionary;
use crate::MaybeOwned;
use derive_new::new;
use itertools::join;
use sysinfo::ProcessExt;

#[derive(Debug)]
pub struct Process {
    inner: sysinfo::Process,
    dict: Dictionary,
}

impl Process {
    crate fn new(inner: sysinfo::Process) -> Process {
        let mut dict = Dictionary::default();
        dict.add("name", Value::string(inner.name()));

        let cmd = inner.cmd();

        let cmd_value = if cmd.len() == 0 {
            Value::nothing()
        } else {
            Value::string(join(cmd, ""))
        };

        dict.add("cmd", cmd_value);
        dict.add("pid", Value::int(inner.pid() as i64));
        dict.add("status", Value::int(inner.status() as i64));

        Process { inner, dict }
    }
}

impl ShellObject for Process {
    fn to_shell_string(&self) -> String {
        format!("{} - {}", self.inner.name(), self.inner.pid())
    }

    fn data_descriptors(&self) -> Vec<DataDescriptor> {
        self.dict.data_descriptors()
    }

    fn get_data(&'a self, desc: &DataDescriptor) -> MaybeOwned<'a, Value> {
        self.dict.get_data(desc)
    }
}
