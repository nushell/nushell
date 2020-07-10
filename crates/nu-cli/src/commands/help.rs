use crate::commands::WholeStreamCommand;
use crate::data::command_dict;

use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{
    NamedType, PositionalType, ReturnSuccess, Signature, SyntaxShape, TaggedDictBuilder,
    UntaggedValue,
};
use nu_source::{SpannedItem, Tagged};
use nu_value_ext::get_data_by_key;

use std::collections::HashMap;

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
            let generated_dir = std::path::Path::new("docs/generated");
            let first_part = std::fs::read_to_string(&generated_dir.join("_skeleton.md"))
                .expect("Skeleton is missing!");
            let second_part = generate_docs(&registry);
            let docs_path = generated_dir.join("documentation.md");
            std::fs::write(&docs_path, first_part + "\n" + &second_part)?;

            Ok(OutputStream::one(ReturnSuccess::value(
                UntaggedValue::string(format!(
                    "Docs generated in {}",
                    docs_path.to_string_lossy().to_string()
                )),
            )))
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

fn indent(s: &str, count: usize) -> String {
    let mut v = s.split('\n').map(|s| s.to_owned()).collect_vec();

    #[allow(clippy::needless_range_loop)]
    for i in 0..v.len() {
        v[i] = format!("{:indent$}{}", "", v[i], indent = count);
    }
    v.join("\n")
}

struct DocumentationConfig {
    no_subcommands: bool,
    no_colour: bool,
}

impl Default for DocumentationConfig {
    fn default() -> Self {
        DocumentationConfig {
            no_subcommands: false,
            no_colour: false,
        }
    }
}

// generate_docs gets the documentation from each command but
fn generate_docs(registry: &CommandRegistry) -> String {
    let mut sorted_names = registry.names();
    sorted_names.sort();

    // cmap will map parent commands to it's subcommands e.g. to -> [to csv, to yaml, to bson]
    let mut cmap: HashMap<String, Vec<String>> = HashMap::new();
    for name in &sorted_names {
        if name.contains(' ') {
            let split_name = name.split_whitespace().collect_vec();
            let parent_name = split_name.first().expect("Expected a parent command name");
            let sub_names = cmap
                .get_mut(*parent_name)
                .expect("Expected a entry for parent");
            sub_names.push(name.to_owned());
        } else {
            cmap.insert(name.to_owned(), Vec::new());
        };
    }
    // Return documentation for each command under a collapsible markdown tag
    // Subcommands are nested under there parent command
    sorted_names
        .iter()
        .fold("".to_owned(), |acc, name| {
            // Must be a sub-command, skip since it's being handled underneath when we hit the parent command
            if !cmap.contains_key(name) {
                return acc;
            }

            let command = registry.get_command(name).unwrap_or_else(|| {
                panic!("Expected command from names to be in registry {}", name)
            });
            // Iterate over all the subcommands, so that we can have a collapsible within a collapsible
            let subcommands_docs = cmap.get(name).unwrap_or(&Vec::new()).iter().fold(
                "".to_owned(),
                |sub_acc, name| {
                    sub_acc
                        + &format!(
                            "- <details><summary>[{}](/commands/{}.html)</summary>\n\n{}\n",
                            name,
                            name.split_whitespace().join("-"), // Because .replace(" ", "-") didn't work
                            indent(
                                &(get_documentation(
                                    command.stream_command(),
                                    registry,
                                    &DocumentationConfig {
                                        no_subcommands: true,
                                        no_colour: true,
                                    }
                                ) + "</details>"), // Kind of dumb but I need to indent </details> as well to get bulleted lists to work
                                2
                            )
                        )
                },
            );

            acc + &format!(
                "<details><summary>[{}](/commands/{}.html)</summary>\n\n{}\n{}</details>\n",
                name,
                name,
                &get_documentation(
                    command.stream_command(),
                    registry,
                    &DocumentationConfig {
                        no_subcommands: true,
                        no_colour: true,
                    }
                ),
                subcommands_docs,
            )
        })
        .replace("\n", "    \n") // To get proper markdown formatting
}

#[allow(clippy::cognitive_complexity)]
fn get_documentation(
    cmd: &dyn WholeStreamCommand,
    registry: &CommandRegistry,
    config: &DocumentationConfig,
) -> String {
    let cmd_name = cmd.name();
    let signature = cmd.signature();
    let mut long_desc = String::new();

    long_desc.push_str(&cmd.usage());
    long_desc.push_str("\n");

    let mut subcommands = String::new();
    if !config.no_subcommands {
        for name in registry.names() {
            if name.starts_with(&format!("{} ", cmd_name)) {
                let subcommand = registry.get_command(&name).expect("This shouldn't happen");

                subcommands.push_str(&format!("  {} - {}\n", name, subcommand.usage()));
            }
        }
    }

    let mut one_liner = String::new();
    one_liner.push_str(&signature.name);
    one_liner.push_str(" ");

    for positional in &signature.positional {
        match &positional.0 {
            PositionalType::Mandatory(name, _m) => {
                one_liner.push_str(&format!("<{}> ", name));
            }
            PositionalType::Optional(name, _o) => {
                one_liner.push_str(&format!("({}) ", name));
            }
        }
    }

    if signature.rest_positional.is_some() {
        one_liner.push_str(" ...args");
    }

    if !subcommands.is_empty() {
        one_liner.push_str("<subcommand> ");
    }

    if !signature.named.is_empty() {
        one_liner.push_str("{flags} ");
    }

    long_desc.push_str(&format!("\nUsage:\n  > {}\n", one_liner));

    if !subcommands.is_empty() {
        long_desc.push_str("\nSubcommands:\n");
        long_desc.push_str(&subcommands);
    }

    if !signature.positional.is_empty() || signature.rest_positional.is_some() {
        long_desc.push_str("\nParameters:\n");
        for positional in &signature.positional {
            match &positional.0 {
                PositionalType::Mandatory(name, _m) => {
                    long_desc.push_str(&format!("  <{}> {}\n", name, positional.1));
                }
                PositionalType::Optional(name, _o) => {
                    long_desc.push_str(&format!("  ({}) {}\n", name, positional.1));
                }
            }
        }

        if let Some(rest_positional) = &signature.rest_positional {
            long_desc.push_str(&format!("  ...args: {}\n", rest_positional.1));
        }
    }
    if !signature.named.is_empty() {
        long_desc.push_str(&get_flags_section(&signature))
    }

    let palette = crate::shell::palette::DefaultPalette {};
    let examples = cmd.examples();
    if !examples.is_empty() {
        long_desc.push_str("\nExamples:");
    }
    for example in examples {
        long_desc.push_str("\n");
        long_desc.push_str("  ");
        long_desc.push_str(example.description);

        if config.no_colour {
            long_desc.push_str(&format!("\n  > {}\n", example.example));
        } else {
            let colored_example =
                crate::shell::helper::Painter::paint_string(example.example, registry, &palette);
            long_desc.push_str(&format!("\n  > {}\n", colored_example));
        }
    }

    long_desc.push_str("\n");

    long_desc
}

pub fn get_help(cmd: &dyn WholeStreamCommand, registry: &CommandRegistry) -> String {
    get_documentation(cmd, registry, &DocumentationConfig::default())
}

fn get_flags_section(signature: &Signature) -> String {
    let mut long_desc = String::new();
    long_desc.push_str("\nFlags:\n");
    for (flag, ty) in &signature.named {
        let msg = match ty.0 {
            NamedType::Switch(s) => {
                if let Some(c) = s {
                    format!(
                        "  -{}, --{}{} {}\n",
                        c,
                        flag,
                        if !ty.1.is_empty() { ":" } else { "" },
                        ty.1
                    )
                } else {
                    format!(
                        "  --{}{} {}\n",
                        flag,
                        if !ty.1.is_empty() { ":" } else { "" },
                        ty.1
                    )
                }
            }
            NamedType::Mandatory(s, m) => {
                if let Some(c) = s {
                    format!(
                        "  -{}, --{} <{}> (required parameter){} {}\n",
                        c,
                        flag,
                        m.display(),
                        if !ty.1.is_empty() { ":" } else { "" },
                        ty.1
                    )
                } else {
                    format!(
                        "  --{} <{}> (required parameter){} {}\n",
                        flag,
                        m.display(),
                        if !ty.1.is_empty() { ":" } else { "" },
                        ty.1
                    )
                }
            }
            NamedType::Optional(s, o) => {
                if let Some(c) = s {
                    format!(
                        "  -{}, --{} <{}>{} {}\n",
                        c,
                        flag,
                        o.display(),
                        if !ty.1.is_empty() { ":" } else { "" },
                        ty.1
                    )
                } else {
                    format!(
                        "  --{} <{}>{} {}\n",
                        flag,
                        o.display(),
                        if !ty.1.is_empty() { ":" } else { "" },
                        ty.1
                    )
                }
            }
        };
        long_desc.push_str(&msg);
    }
    long_desc
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
