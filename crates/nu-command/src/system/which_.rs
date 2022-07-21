use itertools::Itertools;
use log::trace;
use nu_engine::env;
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, IntoInterruptiblePipelineData, PipelineData, ShellError, Signature, Span,
    Spanned, SyntaxShape, Value,
};

use std::ffi::OsStr;
use std::path::Path;

#[derive(Clone)]
pub struct Which;

impl Command for Which {
    fn name(&self) -> &str {
        "which"
    }

    fn signature(&self) -> Signature {
        Signature::build("which")
            .required("application", SyntaxShape::String, "application")
            .rest("rest", SyntaxShape::String, "additional applications")
            .switch("all", "list all executables", Some('a'))
            .category(Category::System)
    }

    fn usage(&self) -> &str {
        "Finds a program file, alias or custom command."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["find", "path", "location", "command"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        which(engine_state, stack, call)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Find if the 'myapp' application is available",
            example: "which myapp",
            result: None,
        }]
    }
}

// Shortcut for creating an entry to the output table
fn entry(arg: impl Into<String>, path: impl Into<String>, builtin: bool, span: Span) -> Value {
    let mut cols = vec![];
    let mut vals = vec![];

    cols.push("arg".to_string());
    vals.push(Value::string(arg.into(), span));

    cols.push("path".to_string());
    vals.push(Value::string(path.into(), span));

    cols.push("built-in".to_string());
    vals.push(Value::Bool { val: builtin, span });

    Value::Record { cols, vals, span }
}

fn get_entry_in_aliases(engine_state: &EngineState, name: &str, span: Span) -> Option<Value> {
    if let Some(alias_id) = engine_state.find_alias(name.as_bytes(), &[]) {
        let alias = engine_state.get_alias(alias_id);
        let alias_str = alias
            .iter()
            .map(|alias_span| String::from_utf8_lossy(engine_state.get_span_contents(alias_span)))
            .join(" ");

        trace!("Found alias: {}", name);

        Some(entry(
            name,
            format!("Nushell alias: {}", alias_str),
            false,
            span,
        ))
    } else {
        None
    }
}

fn get_entry_in_commands(engine_state: &EngineState, name: &str, span: Span) -> Option<Value> {
    if let Some(decl_id) = engine_state.find_decl(name.as_bytes(), &[]) {
        let (msg, is_builtin) = if engine_state.get_decl(decl_id).is_custom_command() {
            ("Nushell custom command", false)
        } else {
            ("Nushell built-in command", true)
        };

        trace!("Found command: {}", name);

        Some(entry(name, msg, is_builtin, span))
    } else {
        None
    }
}

fn get_entries_in_nu(
    engine_state: &EngineState,
    name: &str,
    span: Span,
    skip_after_first_found: bool,
) -> Vec<Value> {
    let mut all_entries = vec![];

    if let Some(ent) = get_entry_in_aliases(engine_state, name, span) {
        all_entries.push(ent);
    }

    if !all_entries.is_empty() && skip_after_first_found {
        return all_entries;
    }

    if let Some(ent) = get_entry_in_commands(engine_state, name, span) {
        all_entries.push(ent);
    }

    all_entries
}

#[cfg(feature = "which-support")]
fn get_first_entry_in_path(
    item: &str,
    span: Span,
    cwd: impl AsRef<Path>,
    paths: impl AsRef<OsStr>,
) -> Option<Value> {
    which::which_in(item, Some(paths), cwd)
        .map(|path| entry(item, path.to_string_lossy().to_string(), false, span))
        .ok()
}

#[cfg(not(feature = "which-support"))]
fn get_first_entry_in_path(
    _item: &str,
    _span: Span,
    _cwd: impl AsRef<Path>,
    _paths: impl AsRef<OsStr>,
) -> Option<Value> {
    None
}

#[cfg(feature = "which-support")]
fn get_all_entries_in_path(
    item: &str,
    span: Span,
    cwd: impl AsRef<Path>,
    paths: impl AsRef<OsStr>,
) -> Vec<Value> {
    which::which_in_all(&item, Some(paths), cwd)
        .map(|iter| {
            iter.map(|path| entry(item, path.to_string_lossy().to_string(), false, span))
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(not(feature = "which-support"))]
fn get_all_entries_in_path(
    _item: &str,
    _span: Span,
    _cwd: impl AsRef<Path>,
    _paths: impl AsRef<OsStr>,
) -> Vec<Value> {
    vec![]
}

#[derive(Debug)]
struct WhichArgs {
    applications: Vec<Spanned<String>>,
    all: bool,
}

fn which_single(
    application: Spanned<String>,
    all: bool,
    engine_state: &EngineState,
    cwd: impl AsRef<Path>,
    paths: impl AsRef<OsStr>,
) -> Vec<Value> {
    let (external, prog_name) = if application.item.starts_with('^') {
        (true, application.item[1..].to_string())
    } else {
        (false, application.item.clone())
    };

    //If prog_name is an external command, don't search for nu-specific programs
    //If all is false, we can save some time by only searching for the first matching
    //program
    //This match handles all different cases
    match (all, external) {
        (true, true) => get_all_entries_in_path(&prog_name, application.span, cwd, paths),
        (true, false) => {
            let mut output: Vec<Value> = vec![];
            output.extend(get_entries_in_nu(
                engine_state,
                &prog_name,
                application.span,
                false,
            ));
            output.extend(get_all_entries_in_path(
                &prog_name,
                application.span,
                cwd,
                paths,
            ));
            output
        }
        (false, true) => {
            if let Some(entry) = get_first_entry_in_path(&prog_name, application.span, cwd, paths) {
                return vec![entry];
            }
            vec![]
        }
        (false, false) => {
            let nu_entries = get_entries_in_nu(engine_state, &prog_name, application.span, true);
            if !nu_entries.is_empty() {
                return vec![nu_entries[0].clone()];
            } else if let Some(entry) =
                get_first_entry_in_path(&prog_name, application.span, cwd, paths)
            {
                return vec![entry];
            }
            vec![]
        }
    }
}

fn which(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<PipelineData, ShellError> {
    let which_args = WhichArgs {
        applications: call.rest(engine_state, stack, 0)?,
        all: call.has_flag("all"),
    };
    let ctrlc = engine_state.ctrlc.clone();

    if which_args.applications.is_empty() {
        return Err(ShellError::MissingParameter(
            "application".into(),
            call.head,
        ));
    }

    let mut output = vec![];

    let cwd = env::current_dir_str(engine_state, stack)?;
    let paths = env::path_str(engine_state, stack, call.head)?;

    for app in which_args.applications {
        let values = which_single(app, which_args.all, engine_state, &cwd, &paths);
        output.extend(values);
    }

    Ok(output.into_iter().into_pipeline_data(ctrlc))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        crate::test_examples(Which)
    }
}
