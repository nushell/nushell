//! Reads the JSON that `carapace <tool> export` prints: the tool's whole command tree, with
//! typed flags and descriptions.

use super::spec::{CommandSpec, FlagSpec, is_command_word};
use nu_protocol::SyntaxShape;
use serde::Deserialize;
use std::collections::HashSet;

#[derive(Deserialize, Default)]
#[serde(rename_all = "PascalCase", default)]
struct Command {
    name: String,
    short: String,
    aliases: Vec<String>,
    hidden: bool,
    commands: Vec<Command>,
    local_flags: Vec<Flag>,
    /// Flags that the subcommands accept too.
    persistent_flags: Vec<Flag>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "PascalCase", default)]
struct Flag {
    longhand: String,
    shorthand: String,
    usage: String,
    #[serde(rename = "Type")]
    ty: String,
    /// The value an optional-value flag gets when it is given without one.
    no_opt_def_val: String,
    hidden: bool,
}

/// Reads the export of the tool, starting at the subcommand `path` (empty for the tool
/// itself). Returns no specs when carapace has no spec for the tool or the subcommand.
pub(super) fn parse(json: &str, path: &[String]) -> Result<Vec<CommandSpec>, serde_json::Error> {
    let root: Command = serde_json::from_str(json)?;
    if root.name.is_empty() {
        return Ok(vec![]);
    }

    let mut command = &root;
    // The persistent flags of the commands above `path`.
    let mut inherited: Vec<&Flag> = vec![];
    for word in path {
        let Some(sub) = command
            .commands
            .iter()
            .find(|sub| &sub.name == word || sub.aliases.contains(word))
        else {
            return Ok(vec![]);
        };
        inherited.extend(&command.persistent_flags);
        command = sub;
    }

    let mut specs = vec![];
    collect(command, path.to_vec(), &inherited, &mut specs);
    // An alias can repeat the name of another command; the first one keeps the name.
    let mut seen = HashSet::new();
    specs.retain(|spec| seen.insert(spec.path.clone()));
    Ok(specs)
}

/// Adds the spec of `command` at `path`, then those of its aliases and its subcommands.
fn collect(
    command: &Command,
    path: Vec<String>,
    inherited: &[&Flag],
    specs: &mut Vec<CommandSpec>,
) {
    let mut inherited = inherited.to_vec();
    inherited.extend(&command.persistent_flags);

    let mut spec = CommandSpec::new(path.clone());
    spec.description.clone_from(&command.short);
    for flag in command.local_flags.iter().chain(inherited.iter().copied()) {
        add_flag(&mut spec, flag);
    }

    // An alias gets the same signature under its own name.
    let aliases = match path.split_last() {
        Some((name, parent)) => command
            .aliases
            .iter()
            .filter(|alias| *alias != name && is_command_word(alias))
            .map(|alias| CommandSpec {
                path: parent.iter().chain([alias]).cloned().collect(),
                ..spec.clone()
            })
            .collect(),
        None => vec![],
    };
    specs.push(spec);
    specs.extend(aliases);

    for sub in &command.commands {
        if !sub.hidden && is_command_word(&sub.name) {
            let mut sub_path = path.clone();
            sub_path.push(sub.name.clone());
            collect(sub, sub_path, &inherited, specs);
        }
    }
}

fn add_flag(spec: &mut CommandSpec, flag: &Flag) {
    if flag.hidden || (flag.longhand.is_empty() && flag.shorthand.is_empty()) {
        return;
    }
    let arg = match flag.ty.as_str() {
        // A flag whose value is optional can't be declared: as a switch it rejects `--flag=x`,
        // and with a type it would take the next argument as its value.
        _ if !flag.no_opt_def_val.is_empty() && !matches!(flag.ty.as_str(), "bool" | "count") => {
            spec.skip(if flag.longhand.is_empty() {
                format!("-{}", flag.shorthand)
            } else {
                format!("--{}", flag.longhand)
            });
            return;
        }
        // An unknown type is declared as a switch. A value after it still reaches the tool in
        // order, while a typed flag would take an argument that isn't its value.
        "" | "bool" | "count" => None,
        "int" | "int8" | "int16" | "int32" | "int64" | "uint" | "uint8" | "uint16" | "uint32"
        | "uint64" => Some(SyntaxShape::Int),
        "float32" | "float64" => Some(SyntaxShape::Number),
        _ => Some(SyntaxShape::String),
    };
    let names = |name: &str| {
        (!name.is_empty())
            .then(|| name.to_string())
            .into_iter()
            .collect()
    };
    spec.add_names(
        names(&flag.longhand),
        names(&flag.shorthand),
        FlagSpec {
            arg,
            description: flag.usage.clone(),
            ..FlagSpec::default()
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    const GIT: &str = r#"{
        "Name": "git",
        "Short": "the stupid content tracker",
        "PersistentFlags": [
            {"Longhand": "exec-path", "Usage": "path to wherever your core git programs are installed", "Type": "string", "NoOptDefVal": " "},
            {"Longhand": "no-pager", "Shorthand": "P", "Usage": "do not pipe git output into a pager", "Type": "bool"}
        ],
        "Commands": [
            {
                "Name": "add",
                "Short": "Add file contents to the index",
                "Aliases": ["stage"],
                "LocalFlags": [
                    {"Longhand": "all", "Shorthand": "A", "Usage": "add all", "Type": "bool"},
                    {"Longhand": "chmod", "Usage": "override the executable bit", "Type": "string"},
                    {"Longhand": "depth", "Usage": "depth", "Type": "int"}
                ]
            },
            {"Name": "__complete", "Hidden": true}
        ]
    }"#;

    #[test]
    fn reads_the_command_tree() -> Result<(), serde_json::Error> {
        let specs = parse(GIT, &[])?;
        let names = specs
            .iter()
            .map(|spec| spec.path.join(" "))
            .collect::<Vec<_>>();
        assert_eq!(names, ["", "add", "stage"]);

        let add = &specs[1];
        assert_eq!(add.description, "Add file contents to the index");
        let flags = add
            .flags
            .iter()
            .map(|flag| (flag.long.as_deref(), flag.short, flag.arg.clone()))
            .collect::<Vec<_>>();
        assert_eq!(
            flags,
            [
                (Some("all"), Some('A'), None),
                (Some("chmod"), None, Some(SyntaxShape::String)),
                (Some("depth"), None, Some(SyntaxShape::Int)),
                (Some("no-pager"), Some('P'), None),
            ]
        );
        // `--exec-path` takes an optional value.
        assert_eq!(add.skipped, ["--exec-path"]);
        Ok(())
    }

    #[test]
    fn starts_at_a_subcommand() -> Result<(), serde_json::Error> {
        let specs = parse(GIT, &["stage".into()])?;
        assert_eq!(specs.len(), 1);
        assert_eq!(specs[0].path, ["stage"]);
        assert!(parse(GIT, &["nope".into()])?.is_empty());
        Ok(())
    }
}
