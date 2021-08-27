use crate::prelude::*;
use indexmap::map::IndexMap;
use log::trace;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Primitive, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tagged;

pub struct Which;

impl WholeStreamCommand for Which {
    fn name(&self) -> &str {
        "which"
    }

    fn signature(&self) -> Signature {
        Signature::build("which")
            .required("application", SyntaxShape::String, "application")
            .rest("rest", SyntaxShape::String, "additional applications")
            .switch("all", "list all executables", Some('a'))
    }

    fn usage(&self) -> &str {
        "Finds a program file, alias or custom command."
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        which(args)
    }
}

/// Shortcuts for creating an entry to the output table
fn entry(arg: impl Into<String>, path: Value, builtin: bool, tag: Tag) -> Value {
    let mut map = IndexMap::new();
    map.insert(
        "arg".to_string(),
        UntaggedValue::Primitive(Primitive::String(arg.into())).into_value(tag.clone()),
    );
    map.insert("path".to_string(), path);
    map.insert(
        "builtin".to_string(),
        UntaggedValue::boolean(builtin).into_value(tag.clone()),
    );

    UntaggedValue::row(map).into_value(tag)
}

macro_rules! create_entry {
    ($arg:expr, $path:expr, $tag:expr, $is_builtin:expr) => {
        entry(
            $arg.clone(),
            UntaggedValue::Primitive(Primitive::String($path.to_string())).into_value($tag.clone()),
            $is_builtin,
            $tag,
        )
    };
}

fn get_entries_in_aliases(scope: &Scope, name: &str, tag: Tag) -> Vec<Value> {
    let aliases = scope
        .get_aliases_with_name(name)
        .unwrap_or_default()
        .into_iter()
        .map(|spans| {
            spans
                .into_iter()
                .map(|span| span.item)
                .collect::<Vec<String>>()
                .join(" ")
        })
        .map(|alias| {
            create_entry!(
                name,
                format!("Nushell alias: {}", alias),
                tag.clone(),
                false
            )
        })
        .collect::<Vec<_>>();
    trace!("Found {} aliases", aliases.len());
    aliases
}

fn get_entries_in_custom_command(scope: &Scope, name: &str, tag: Tag) -> Vec<Value> {
    scope
        .get_custom_commands_with_name(name)
        .unwrap_or_default()
        .into_iter()
        .map(|_| create_entry!(name, "Nushell custom command", tag.clone(), false))
        .collect()
}

fn get_entry_in_commands(scope: &Scope, name: &str, tag: Tag) -> Option<Value> {
    if scope.has_command(name) {
        Some(create_entry!(name, "Nushell built-in command", tag, true))
    } else {
        None
    }
}

fn get_entries_in_nu(
    scope: &Scope,
    name: &str,
    tag: Tag,
    skip_after_first_found: bool,
) -> Vec<Value> {
    let mut all_entries = vec![];

    all_entries.extend(get_entries_in_aliases(scope, name, tag.clone()));
    if !all_entries.is_empty() && skip_after_first_found {
        return all_entries;
    }

    all_entries.extend(get_entries_in_custom_command(scope, name, tag.clone()));
    if !all_entries.is_empty() && skip_after_first_found {
        return all_entries;
    }

    if let Some(entry) = get_entry_in_commands(scope, name, tag) {
        all_entries.push(entry);
    }

    all_entries
}

#[allow(unused)]
macro_rules! entry_path {
    ($arg:expr, $path:expr, $tag:expr) => {
        entry(
            $arg.clone(),
            UntaggedValue::Primitive(Primitive::FilePath($path)).into_value($tag.clone()),
            false,
            $tag,
        )
    };
}

#[cfg(feature = "which")]
fn get_first_entry_in_path(item: &str, tag: Tag) -> Option<Value> {
    which::which(item)
        .map(|path| entry_path!(item, path, tag))
        .ok()
}

#[cfg(not(feature = "which"))]
fn get_first_entry_in_path(_: &str, _: Tag) -> Option<Value> {
    None
}

#[cfg(feature = "which")]
fn get_all_entries_in_path(item: &str, tag: Tag) -> Vec<Value> {
    which::which_all(&item)
        .map(|iter| {
            iter.map(|path| entry_path!(item, path, tag.clone()))
                .collect()
        })
        .unwrap_or_default()
}
#[cfg(not(feature = "which"))]
fn get_all_entries_in_path(_: &str, _: Tag) -> Vec<Value> {
    vec![]
}

#[derive(Debug)]
struct WhichArgs {
    applications: Vec<Tagged<String>>,
    all: bool,
}

fn which_single(application: Tagged<String>, all: bool, scope: &Scope) -> Vec<Value> {
    let (external, prog_name) = if application.starts_with('^') {
        (true, application.item[1..].to_string())
    } else {
        (false, application.item.clone())
    };

    //If prog_name is an external command, don't search for nu-specific programs
    //If all is false, we can save some time by only searching for the first matching
    //program
    //This match handles all different cases
    match (all, external) {
        (true, true) => get_all_entries_in_path(&prog_name, application.tag),
        (true, false) => {
            let mut output: Vec<Value> = vec![];
            output.extend(get_entries_in_nu(
                scope,
                &prog_name,
                application.tag.clone(),
                false,
            ));
            output.extend(get_all_entries_in_path(&prog_name, application.tag));
            output
        }
        (false, true) => {
            if let Some(entry) = get_first_entry_in_path(&prog_name, application.tag) {
                return vec![entry];
            }
            vec![]
        }
        (false, false) => {
            let nu_entries = get_entries_in_nu(scope, &prog_name, application.tag.clone(), true);
            if !nu_entries.is_empty() {
                return vec![nu_entries[0].clone()];
            } else if let Some(entry) = get_first_entry_in_path(&prog_name, application.tag) {
                return vec![entry];
            }
            vec![]
        }
    }
}

fn which(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let which_args = WhichArgs {
        applications: args.rest_with_minimum(0, 1)?,
        all: args.has_flag("all"),
    };

    let mut output = vec![];

    for app in which_args.applications {
        let values = which_single(app, which_args.all, args.scope());
        output.extend(values);
    }

    Ok(output.into_iter().into_output_stream())
}

#[cfg(test)]
mod tests {
    use super::ShellError;
    use super::Which;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Which {})
    }
}
