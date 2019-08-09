use crate::errors::ShellError;
use crate::object::process::process_dict;
use crate::prelude::*;
use sysinfo::{RefreshKind, SystemExt};

pub fn ps(args: CommandArgs, _registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let mut system = sysinfo::System::new_with_specifics(RefreshKind::new().with_processes());
    system.refresh_processes();
    let list = system.get_process_list();

    let list = list
        .into_iter()
        .map(|(_, process)| process_dict(process, Tag::unknown_origin(args.call_info.name_span)))
        .collect::<VecDeque<_>>();

    Ok(list.from_input_stream())
}
