use crate::errors::ShellError;
use crate::object::process::Process;
use crate::object::{ShellObject, Value};
use derive_new::new;
use sysinfo::SystemExt;

#[derive(new)]
pub struct Ps {
    system: sysinfo::System,
}

impl crate::Command for Ps {
    fn run(
        &mut self,
        _host: &dyn crate::Host,
        _env: &mut crate::Environment,
    ) -> Result<Value, ShellError> {
        self.system.refresh_all();

        let list = self.system.get_process_list();

        let list = list
            .into_iter()
            .map(|(_, process)| Value::Object(Box::new(Process::new(process.clone()))))
            .take(5)
            .collect();

        Ok(Value::List(list))
    }
}
