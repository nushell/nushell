use crate::commands::WholeStreamCommand;
use crate::data::command_dict;
use crate::documentation::{generate_docs, get_documentation, DocumentationConfig};

use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, SyntaxShape, TaggedDictBuilder, UntaggedValue};
use nu_source::{SpannedItem, Tagged};
use nu_value_ext::get_data_by_key;

pub struct Help;

#[derive(Deserialize)]
pub struct HelpArgs {
    rest: Vec<Tagged<String>>,
}

#[async_trait]
impl WholeStreamCommand for Help {
    fn name(&self) -> &str {
        "help"
    }

    fn signature(&self) -> Signature {
        Signature::build("help").rest(SyntaxShape::String, "the name of command to get help on")
    }

    fn usage(&self) -> &str {
        "Display help information about commands."
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        help(args, registry).await
    }
}

async fn help(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let name = args.call_info.name_tag.clone();
    let (HelpArgs { rest }, ..) = args.process(&registry).await?;

    if !rest.is_empty() {
        if rest[0].item == "commands" {
            let mut sorted_names = registry.names();
            sorted_names.sort();

            Ok(
                futures::stream::iter(sorted_names.into_iter().filter_map(move |cmd| {
                    // If it's a subcommand, don't list it during the commands list
                    if cmd.contains(' ') {
                        return None;
                    }
                    let mut short_desc = TaggedDictBuilder::new(name.clone());
                    let document_tag = rest[0].tag.clone();
                    let value = command_dict(
                        match registry.get_command(&cmd).ok_or_else(|| {
                            ShellError::labeled_error(
                                format!("Could not load {}", cmd),
                                "could not load command",
                                document_tag,
                            )
                        }) {
                            Ok(ok) => ok,
                            Err(err) => return Some(Err(err)),
                        },
                        name.clone(),
                    );

                    short_desc.insert_untagged("name", cmd);
                    short_desc.insert_untagged(
                        "description",
                        match match get_data_by_key(&value, "usage".spanned_unknown()).ok_or_else(
                            || {
                                ShellError::labeled_error(
                                    "Expected a usage key",
                                    "expected a 'usage' key",
                                    &value.tag,
                                )
                            },
                        ) {
                            Ok(ok) => ok,
                            Err(err) => return Some(Err(err)),
                        }
                        .as_string()
                        {
                            Ok(ok) => ok,
                            Err(err) => return Some(Err(err)),
                        },
                    );

                    Some(ReturnSuccess::value(short_desc.into_value()))
                }))
                .to_output_stream(),
            )
        } else if rest[0].item == "generate_docs" {
            Ok(OutputStream::one(ReturnSuccess::value(generate_docs(
                &registry,
            ))))
        } else if rest.len() == 2 {
            // Check for a subcommand
            let command_name = format!("{} {}", rest[0].item, rest[1].item);
            if let Some(command) = registry.get_command(&command_name) {
                Ok(OutputStream::one(ReturnSuccess::value(
                    UntaggedValue::string(get_help(command.stream_command(), &registry))
                        .into_value(Tag::unknown()),
                )))
            } else {
                Ok(OutputStream::empty())
            }
        } else if let Some(command) = registry.get_command(&rest[0].item) {
            Ok(OutputStream::one(ReturnSuccess::value(
                UntaggedValue::string(get_help(command.stream_command(), &registry))
                    .into_value(Tag::unknown()),
            )))
        } else {
            Err(ShellError::labeled_error(
                "Can't find command (use 'help commands' for full list)",
                "can't find command",
                rest[0].tag.span,
            ))
        }
    } else {
        let msg = r#"Welcome to Nushell.

Here are some tips to help you get started.
  * help commands - list all available commands
  * help <command name> - display help about a particular command

Nushell works on the idea of a "pipeline". Pipelines are commands connected with the '|' character.
Each stage in the pipeline works together to load, parse, and display information to you.

[Examples]

List the files in the current directory, sorted by size:
    ls | sort-by size

Get information about the current system:
    sys | get host

Get the processes on your system actively using CPU:
    ps | where cpu > 0

You can also learn more at https://www.nushell.sh/book/"#;

        Ok(OutputStream::one(ReturnSuccess::value(
            UntaggedValue::string(msg).into_value(Tag::unknown()),
        )))
    }
}

pub fn get_help(cmd: &dyn WholeStreamCommand, registry: &CommandRegistry) -> String {
    get_documentation(cmd, registry, &DocumentationConfig::default())
}

#[cfg(test)]
mod tests {
    use super::Help;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(Help {})
    }
}
