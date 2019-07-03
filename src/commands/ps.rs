use crate::errors::ShellError;
use crate::object::process::process_dict;
use crate::object::Value;
use crate::prelude::*;
use sysinfo::{RefreshKind, SystemExt};

pub fn ps(_args: CommandArgs) -> Result<OutputStream, ShellError> {
    let mut system = sysinfo::System::new_with_specifics(RefreshKind::new().with_processes());
    system.refresh_processes();
    let list = system.get_process_list();

    let list = list
        .into_iter()
        .map(|(_, process)| Value::Object(process_dict(process)))
        .collect::<VecDeque<_>>();

    Ok(list.from_input_stream())
}
