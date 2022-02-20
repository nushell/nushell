use itertools::Itertools;
use log::trace;
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, IntoInterruptiblePipelineData, PipelineData, ShellError, Signature, Span,
    Spanned, SyntaxShape, Value,
};

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

/// Shortcuts for creating an entry to the output table
fn entry(arg: impl Into<String>, path: Value, builtin: bool, span: Span) -> Value {
    let mut cols = vec![];
    let mut vals = vec![];

    cols.push("arg".to_string());
    vals.push(Value::string(arg.into(), span));

    cols.push("path".to_string());
    vals.push(path);

    cols.push("builtin".to_string());
    vals.push(Value::Bool { val: builtin, span });

    Value::Record { cols, vals, span }
}

macro_rules! create_entry {
    ($arg:expr, $path:expr, $span:expr, $is_builtin:expr) => {
        entry(
            $arg.clone(),
            Value::string($path.to_string(), $span),
            $is_builtin,
            $span,
        )
    };
}

fn get_entries_in_aliases(engine_state: &EngineState, name: &str, span: Span) -> Vec<Value> {
    let aliases = engine_state.find_aliases(name);

    let aliases = aliases
        .into_iter()
        .map(|spans| {
            spans
                .iter()
                .map(|span| {
                    String::from_utf8_lossy(engine_state.get_span_contents(span)).to_string()
                })
                .join(" ")
        })
        .map(|alias| create_entry!(name, format!("Nushell alias: {}", alias), span, false))
        .collect::<Vec<_>>();
    trace!("Found {} aliases", aliases.len());
    aliases
}

fn get_entries_in_custom_command(engine_state: &EngineState, name: &str, span: Span) -> Vec<Value> {
    let custom_commands = engine_state.find_custom_commands(name);

    custom_commands
        .into_iter()
        .map(|_| create_entry!(name, "Nushell custom command", span, false))
        .collect::<Vec<_>>()
}

fn get_entry_in_commands(engine_state: &EngineState, name: &str, span: Span) -> Option<Value> {
    if engine_state.find_decl(name.as_bytes()).is_some() {
        Some(create_entry!(name, "Nushell built-in command", span, true))
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

    all_entries.extend(get_entries_in_aliases(engine_state, name, span));
    if !all_entries.is_empty() && skip_after_first_found {
        return all_entries;
    }

    all_entries.extend(get_entries_in_custom_command(engine_state, name, span));
    if !all_entries.is_empty() && skip_after_first_found {
        return all_entries;
    }

    if let Some(entry) = get_entry_in_commands(engine_state, name, span) {
        all_entries.push(entry);
    }

    all_entries
}

#[allow(unused)]
macro_rules! entry_path {
    ($arg:expr, $path:expr, $span:expr) => {
        entry($arg.clone(), Value::string($path, $span), false, $span)
    };
}

#[cfg(feature = "which")]
fn get_first_entry_in_path(item: &str, span: Span) -> Option<Value> {
    which::which(item)
        .map(|path| entry_path!(item, path.to_string_lossy().to_string(), span))
        .ok()
}

#[cfg(not(feature = "which"))]
fn get_first_entry_in_path(_: &str, _: Span) -> Option<Value> {
    None
}

#[cfg(feature = "which")]
fn get_all_entries_in_path(item: &str, span: Span) -> Vec<Value> {
    which::which_all(&item)
        .map(|iter| {
            iter.map(|path| entry_path!(item, path.to_string_lossy().to_string(), span))
                .collect()
        })
        .unwrap_or_default()
}
#[cfg(not(feature = "which"))]
fn get_all_entries_in_path(_: &str, _: Span) -> Vec<Value> {
    vec![]
}

#[derive(Debug)]
struct WhichArgs {
    applications: Vec<Spanned<String>>,
    all: bool,
}

fn which_single(application: Spanned<String>, all: bool, engine_state: &EngineState) -> Vec<Value> {
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
        (true, true) => get_all_entries_in_path(&prog_name, application.span),
        (true, false) => {
            let mut output: Vec<Value> = vec![];
            output.extend(get_entries_in_nu(
                engine_state,
                &prog_name,
                application.span,
                false,
            ));
            output.extend(get_all_entries_in_path(&prog_name, application.span));
            output
        }
        (false, true) => {
            if let Some(entry) = get_first_entry_in_path(&prog_name, application.span) {
                return vec![entry];
            }
            vec![]
        }
        (false, false) => {
            let nu_entries = get_entries_in_nu(engine_state, &prog_name, application.span, true);
            if !nu_entries.is_empty() {
                return vec![nu_entries[0].clone()];
            } else if let Some(entry) = get_first_entry_in_path(&prog_name, application.span) {
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

    for app in which_args.applications {
        let values = which_single(app, which_args.all, engine_state);
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
