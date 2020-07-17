use crate::commands::WholeStreamCommand;

use crate::prelude::*;
use nu_protocol::{NamedType, PositionalType, Signature, UntaggedValue, Value};

use indexmap::IndexMap;

use std::collections::HashMap;

const COMMANDS_DOCS_DIR: &str = "docs/commands";

pub struct DocumentationConfig {
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

fn generate_doc(registry: &CommandRegistry, name: &str) -> IndexMap<String, Value> {
    let mut row_entries = IndexMap::new();
    let command = registry
        .get_command(name)
        .unwrap_or_else(|| panic!("Expected command '{}' from names to be in registry", name));
    row_entries.insert(
        "name".to_owned(),
        UntaggedValue::string(name).into_untagged_value(),
    );
    row_entries.insert(
        "usage".to_owned(),
        UntaggedValue::string(command.usage()).into_untagged_value(),
    );
    retrieve_doc_link(name).and_then(|link| {
        row_entries.insert(
            "doc_link".to_owned(),
            UntaggedValue::string(link).into_untagged_value(),
        )
    });
    row_entries.insert(
        "documentation".to_owned(),
        UntaggedValue::string(get_documentation(
            command.stream_command(),
            registry,
            &DocumentationConfig {
                no_subcommands: true,
                no_colour: true,
            },
        ))
        .into_untagged_value(),
    );
    row_entries
}

// generate_docs gets the documentation from each command and returns a Table as output
pub fn generate_docs(registry: &CommandRegistry) -> Value {
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
    // Return documentation for each command
    // Sub-commands are nested under there respective parent commands
    let mut table = Vec::new();
    for name in sorted_names.iter() {
        // Must be a sub-command, skip since it's being handled underneath when we hit the parent command
        if !cmap.contains_key(name) {
            continue;
        }
        let mut row_entries = generate_doc(registry, name);
        // Iterate over all the subcommands of the parent command
        let mut sub_table = Vec::new();
        for sub_name in cmap.get(name).unwrap_or(&Vec::new()).iter() {
            let sub_row = generate_doc(registry, sub_name);
            sub_table.push(UntaggedValue::row(sub_row).into_untagged_value());
        }

        if !sub_table.is_empty() {
            row_entries.insert(
                "subcommands".to_owned(),
                UntaggedValue::table(&sub_table).into_untagged_value(),
            );
        }
        table.push(UntaggedValue::row(row_entries).into_untagged_value());
    }
    UntaggedValue::table(&table).into_untagged_value()
}

fn retrieve_doc_link(name: &str) -> Option<String> {
    let doc_name = name.split_whitespace().join("_"); // Because .replace(" ", "_") didn't work
    let mut entries =
        std::fs::read_dir(COMMANDS_DOCS_DIR).expect("Directory for command docs are missing!");
    entries.find_map(|r| {
        r.map_or(None, |de| {
            if de.file_name().to_string_lossy() == format!("{}.{}", &doc_name, "md") {
                Some(format!("/commands/{}.{}", &doc_name, "html"))
            } else {
                None
            }
        })
    })
}

#[allow(clippy::cognitive_complexity)]
pub fn get_documentation(
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
