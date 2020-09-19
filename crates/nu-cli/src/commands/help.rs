use crate::commands::command::Command;
use crate::commands::WholeStreamCommand;
use crate::documentation::{generate_docs, get_documentation, DocumentationConfig};
use crate::prelude::*;
use nu_data::command::signature_dict;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, SyntaxShape, TaggedDictBuilder, UntaggedValue, Value};
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

pub(crate) fn command_dict(command: Command, tag: impl Into<Tag>) -> Value {
    let tag = tag.into();

    let mut cmd_dict = TaggedDictBuilder::new(&tag);

    cmd_dict.insert_untagged("name", UntaggedValue::string(command.name()));

    cmd_dict.insert_untagged("type", UntaggedValue::string("Command"));

    cmd_dict.insert_value("signature", signature_dict(command.signature(), tag));
    cmd_dict.insert_untagged("usage", UntaggedValue::string(command.usage()));

    cmd_dict.into_value()
}

async fn help(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let name = args.call_info.name_tag.clone();
    let (HelpArgs { rest }, ..) = args.process(&registry).await?;

    if !rest.is_empty() {
        if rest[0].item == "commands" {
            let mut sorted_names = registry.names();
            sorted_names.sort();

            let (mut subcommand_names, command_names) = sorted_names
                .into_iter()
                // Internal only commands shouldn't be displayed
                .filter(|cmd_name| {
                    registry
                        .get_command(&cmd_name)
                        .filter(|command| !command.is_internal())
                        .is_some()
                })
                .partition::<Vec<_>, _>(|cmd_name| cmd_name.contains(' '));

            fn process_name(
                dict: &mut TaggedDictBuilder,
                cmd_name: &str,
                registry: CommandRegistry,
                rest: Vec<Tagged<String>>,
                name: Tag,
            ) -> Result<(), ShellError> {
                let document_tag = rest[0].tag.clone();
                let value = command_dict(
                    registry.get_command(&cmd_name).ok_or_else(|| {
                        ShellError::labeled_error(
                            format!("Could not load {}", cmd_name),
                            "could not load command",
                            document_tag,
                        )
                    })?,
                    name,
                );

                dict.insert_untagged("name", cmd_name);
                dict.insert_untagged(
                    "description",
                    get_data_by_key(&value, "usage".spanned_unknown())
                        .ok_or_else(|| {
                            ShellError::labeled_error(
                                "Expected a usage key",
                                "expected a 'usage' key",
                                &value.tag,
                            )
                        })?
                        .as_string()?,
                );

                //ReturnSuccess::value(dict.into_value())
                Ok(())
            }

            fn make_subcommands_table(
                subcommand_names: &mut Vec<String>,
                cmd_name: &str,
                registry: CommandRegistry,
                rest: Vec<Tagged<String>>,
                name: Tag,
            ) -> Result<Value, ShellError> {
                let (matching, not_matching) = subcommand_names
                    .drain(..)
                    .partition(|subcommand_name| subcommand_name.starts_with(cmd_name));
                *subcommand_names = not_matching;
                Ok(if !matching.is_empty() {
                    UntaggedValue::table(
                        &(matching
                            .into_iter()
                            .map(|cmd_name: String| -> Result<_, ShellError> {
                                let mut short_desc = TaggedDictBuilder::new(name.clone());
                                process_name(
                                    &mut short_desc,
                                    &cmd_name,
                                    registry.clone(),
                                    rest.clone(),
                                    name.clone(),
                                )?;
                                Ok(short_desc.into_value())
                            })
                            .collect::<Result<Vec<_>, _>>()?[..]),
                    )
                    .into_value(name)
                } else {
                    UntaggedValue::nothing().into_value(name)
                })
            }

            let iterator =
                command_names
                    .into_iter()
                    .map(move |cmd_name| -> Result<_, ShellError> {
                        let mut short_desc = TaggedDictBuilder::new(name.clone());
                        process_name(
                            &mut short_desc,
                            &cmd_name,
                            registry.clone(),
                            rest.clone(),
                            name.clone(),
                        )?;
                        short_desc.insert_value(
                            "subcommands",
                            make_subcommands_table(
                                &mut subcommand_names,
                                &cmd_name,
                                registry.clone(),
                                rest.clone(),
                                name.clone(),
                            )?,
                        );
                        ReturnSuccess::value(short_desc.into_value())
                    });

            Ok(futures::stream::iter(iterator).to_output_stream())
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
