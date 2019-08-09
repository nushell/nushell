use crate::errors::ShellError;
use crate::object::TaggedDictBuilder;
use crate::prelude::*;

pub fn shells(args: CommandArgs, _registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let mut shells_out = VecDeque::new();
    let span = args.call_info.name_span;

    for shell in args.shell_manager.shells.lock().unwrap().iter() {
        let mut dict = TaggedDictBuilder::new(Tag::unknown_origin(span));
        dict.insert("name", shell.name());
        dict.insert("path", shell.path());

        shells_out.push_back(dict.into_tagged_value());
    }

    Ok(shells_out.to_output_stream())
}
