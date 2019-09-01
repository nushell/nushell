use crate::commands::WholeStreamCommand;
use crate::errors::ShellError;
use crate::object::TaggedDictBuilder;
use crate::prelude::*;

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

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        shells(args, registry)
    }
}

fn shells(args: CommandArgs, _registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let mut shells_out = VecDeque::new();
    let span = args.call_info.name_span;

    for (index, shell) in args.shell_manager.shells.lock().unwrap().iter().enumerate() {
        let mut dict = TaggedDictBuilder::new(Tag::unknown_origin(span));

        if index == args.shell_manager.current_shell {
            dict.insert(" ", "X".to_string());
        } else {
            dict.insert(" ", " ".to_string());
        }
        dict.insert("name", shell.name(&args.call_info.source_map));
        dict.insert("path", shell.path());

        shells_out.push_back(dict.into_tagged_value());
    }

    Ok(shells_out.to_output_stream())
}
