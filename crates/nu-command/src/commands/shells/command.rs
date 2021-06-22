use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, TaggedDictBuilder};
use std::sync::atomic::Ordering;

pub struct Shells;

impl WholeStreamCommand for Shells {
    fn name(&self) -> &str {
        "shells"
    }

    fn signature(&self) -> Signature {
        Signature::build("shells")
    }

    fn usage(&self) -> &str {
        "Display the list of current shells."
    }

    fn run_with_actions(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        Ok(shells(args))
    }
}

fn shells(args: CommandArgs) -> ActionStream {
    let mut shells_out = VecDeque::new();
    let shell_manager = args.shell_manager();
    let tag = args.call_info.name_tag;
    let active_index = shell_manager.current_shell.load(Ordering::SeqCst);

    for (index, shell) in shell_manager.shells.lock().iter().enumerate() {
        let mut dict = TaggedDictBuilder::new(&tag);

        if index == active_index {
            dict.insert_untagged("active", true);
        } else {
            dict.insert_untagged("active", false);
        }
        dict.insert_untagged("name", shell.name());
        dict.insert_untagged("path", shell.path());

        shells_out.push_back(dict.into_value());
    }

    shells_out.into()
}

#[cfg(test)]
mod tests {
    use super::ShellError;
    use super::Shells;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Shells {})
    }
}
