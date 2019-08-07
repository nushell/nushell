use crate::commands::command::CommandAction;
use crate::errors::ShellError;
use crate::parser::registry::{CommandConfig, NamedType};
use crate::prelude::*;
use indexmap::IndexMap;

pub struct Exit;

impl Command for Exit {
    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        exit(args)
    }

    fn name(&self) -> &str {
        "exit"
    }

    fn config(&self) -> CommandConfig {
        let mut named: IndexMap<String, NamedType> = IndexMap::new();
        named.insert("now".to_string(), NamedType::Switch);

        CommandConfig {
            name: self.name().to_string(),
            positional: vec![],
            rest_positional: false,
            named,
            is_sink: false,
            is_filter: false,
        }
    }
}

pub fn exit(args: CommandArgs) -> Result<OutputStream, ShellError> {
    if args.call_info.args.has("now") {
        Ok(vec![Ok(ReturnSuccess::Action(CommandAction::Exit))].into())
    } else {
        Ok(vec![Ok(ReturnSuccess::Action(CommandAction::LeaveShell))].into())
    }
}
