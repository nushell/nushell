use crate::errors::ShellError;
use crate::object::process::process_dict;
use crate::object::Value;
use crate::prelude::*;
use derive_new::new;
use sysinfo::SystemExt;

#[derive(new)]
pub struct Ps;

impl crate::Command for Ps {
    fn run(&self, _args: CommandArgs<'caller>) -> Result<VecDeque<ReturnValue>, ShellError> {
        let mut system = sysinfo::System::new();
        system.refresh_all();

        let list = system.get_process_list();

        let list = list
            .into_iter()
            .map(|(_, process)| ReturnValue::Value(Value::Object(process_dict(process))))
            .collect::<VecDeque<_>>();

        Ok(list)
    }
}
