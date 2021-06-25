use crate::prelude::*;
use indexmap::IndexMap;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    value::StringExt, CommandAction, NamedType, PositionalType, ReturnSuccess, Signature,
    SyntaxShape,
};
use serde::{Deserialize, Serialize};

pub struct Lang;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CommandInfo {
    name: String,
    usage: String,
    params_positional: Vec<(PositionalType, String)>,
    params_rest: Option<(SyntaxShape, String)>,
    params_named: IndexMap<String, (NamedType, String)>,
    is_filter: bool,
    is_builtin: bool,
    is_subcommand: bool,
    is_plugin: bool,
    is_custom_command: bool,
    is_private_command: bool,
    is_binary: bool,
    extra_usage: String,
}

impl WholeStreamCommand for Lang {
    fn name(&self) -> &str {
        "lang"
    }

    fn signature(&self) -> Signature {
        Signature::build("lang")
    }

    fn usage(&self) -> &str {
        "Returns the nushell-lang information"
    }

    fn extra_usage(&self) -> &str {
        ""
    }

    fn run_with_actions(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        let tag = args.call_info.name_tag.clone();

        let mut command_info: Vec<CommandInfo> = vec![];
        let full_commands = args.context.scope.get_commands_info();
        for (_key, cmd) in full_commands {
            let mut sig = cmd.signature();
            sig.remove_named("help");
            command_info.push(CommandInfo {
                name: cmd.name().to_string(),
                usage: cmd.usage().to_string(),
                params_positional: sig.positional,
                params_rest: sig.rest_positional,
                params_named: sig.named,
                is_filter: sig.is_filter,
                is_builtin: cmd.is_builtin(),
                is_subcommand: cmd.name().contains(' '),
                is_plugin: cmd.is_plugin(),
                is_custom_command: cmd.is_custom(),
                is_private_command: cmd.is_private(),
                is_binary: cmd.is_binary(),
                extra_usage: cmd.extra_usage().replace("\\", "\\\\").replace("\n", "\\n"),
            })
        }

        let cmds = serde_json::to_string(&command_info)
            .expect("error converting command info to json string");

        // Ok(ActionStream::one(
        //     UntaggedValue::string(cmds).into_value(tag),
        // ))

        Ok(ActionStream::one(ReturnSuccess::action(
            CommandAction::AutoConvert(cmds.to_string_value(tag), "json".to_string()),
        )))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Query command names from nushell",
            example: "lang",
            result: None,
        }]
    }

    fn run(&self, args: CommandArgs) -> Result<InputStream, ShellError> {
        let context = args.context.clone();
        let stream = self.run_with_actions(args)?;

        Ok(Box::new(nu_engine::evaluate::internal::InternalIterator {
            context,
            input: stream,
            leftovers: InputStream::empty(),
        })
        .into_output_stream())
    }
}
