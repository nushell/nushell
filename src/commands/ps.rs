use crate::commands::WholeStreamCommand;
use crate::errors::ShellError;
use crate::object::process::process_dict;
use crate::prelude::*;
#[allow(unused)]
use sysinfo::{RefreshKind, SystemExt};

pub struct PS;

impl WholeStreamCommand for PS {
    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        ps(args, registry)
    }

    fn name(&self) -> &str {
        "ps"
    }

    fn signature(&self) -> Signature {
        Signature::build("ps")
    }
}

fn ps(args: CommandArgs, _registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let system;

    #[cfg(target_os = "linux")]
    {
        system = sysinfo::System::new();
    }

    #[cfg(not(target_os = "linux"))]
    {
        let mut sy = sysinfo::System::new_with_specifics(RefreshKind::new().with_processes());
        sy.refresh_processes();

        system = sy;
    }
    let list = system.get_process_list();

    let list = list
        .into_iter()
        .map(|(_, process)| process_dict(process, Tag::unknown_origin(args.call_info.name_span)))
        .collect::<VecDeque<_>>();

    Ok(list.from_input_stream())
}
