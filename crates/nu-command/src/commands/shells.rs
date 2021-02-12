use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, TaggedDictBuilder};
use std::sync::atomic::Ordering;

pub struct Shells;

#[async_trait]
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

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        Ok(shells(args))
    }
}

fn shells(args: CommandArgs) -> OutputStream {
    let mut shells_out = VecDeque::new();
    let tag = args.call_info.name_tag;

    for (index, shell) in args.shell_manager.shells.lock().iter().enumerate() {
        let mut dict = TaggedDictBuilder::new(&tag);

        if index == (*args.shell_manager.current_shell).load(Ordering::SeqCst) {
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
