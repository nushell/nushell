use itertools::Itertools;
use nu_engine::{command_prelude::*, env};
use nu_protocol::engine::CommandType;
use std::fs;
use std::{ffi::OsStr, path::Path};
use which::sys;
use which::sys::Sys;

#[derive(Clone)]
pub struct Which;

impl Command for Which {
    fn name(&self) -> &str {
        "which"
    }

    fn signature(&self) -> Signature {
        Signature::build("which")
            .input_output_types(vec![(Type::Nothing, Type::table())])
            .allow_variants_without_examples(true)
            .rest("applications", SyntaxShape::String, "Application(s).")
            .switch("all", "list all executables", Some('a'))
            .category(Category::System)
    }

    fn description(&self) -> &str {
        "Finds a program file, alias or custom command. If `application` is not provided, all deduplicated commands will be returned."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec![
            "find",
            "path",
            "location",
            "command",
            "whereis",     // linux binary to find binary locations in path
            "get-command", // powershell command to find commands and binaries in path
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        which(engine_state, stack, call)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Find if the 'myapp' application is available",
                example: "which myapp",
                result: None,
            },
            Example {
                description: "Find all executables across all paths without deduplication",
                example: "which -a",
                result: None,
            },
        ]
    }
}

// Shortcut for creating an entry to the output table
fn entry(
    arg: impl Into<String>,
    path: impl Into<String>,
    cmd_type: CommandType,
    span: Span,
) -> Value {
    Value::record(
        record! {
            "command" => Value::string(arg, span),
            "path" => Value::string(path, span),
            "type" => Value::string(cmd_type.to_string(), span),
        },
        span,
    )
}

fn get_entry_in_commands(engine_state: &EngineState, name: &str, span: Span) -> Option<Value> {
    if let Some(decl_id) = engine_state.find_decl(name.as_bytes(), &[]) {
        let decl = engine_state.get_decl(decl_id);
        Some(entry(name, "", decl.command_type(), span))
    } else {
        None
    }
}

fn get_first_entry_in_path(
    item: &str,
    span: Span,
    cwd: impl AsRef<Path>,
    paths: impl AsRef<OsStr>,
) -> Option<Value> {
    which::which_in(item, Some(paths), cwd)
        .map(|path| entry(item, path.to_string_lossy(), CommandType::External, span))
        .ok()
}

fn get_all_entries_in_path(
    item: &str,
    span: Span,
    cwd: impl AsRef<Path>,
    paths: impl AsRef<OsStr>,
) -> Vec<Value> {
    which::which_in_all(&item, Some(paths), cwd)
        .map(|iter| {
            iter.map(|path| entry(item, path.to_string_lossy(), CommandType::External, span))
                .collect()
        })
        .unwrap_or_default()
}

fn list_all_executables(
    engine_state: &EngineState,
    paths: impl AsRef<OsStr>,
    all: bool,
) -> Vec<Value> {
    let decls = engine_state.get_decls_sorted(false);
    let commands = decls
        .into_iter()
        .map(|x| {
            let decl = engine_state.get_decl(x.1);
            (
                String::from_utf8_lossy(&x.0).to_string(),
                String::new(),
                decl.command_type(),
            )
        })
        .chain(
            sys::RealSys
                .env_split_paths(paths.as_ref())
                .into_iter()
                .filter_map(|dir| fs::read_dir(dir).ok())
                .flat_map(|entries| entries.flatten())
                .map(|entry| entry.path())
                .filter(|path| path.is_file())
                .filter_map(|path| {
                    let filename = path.file_name()?.to_string_lossy().to_string();
                    Some((
                        filename,
                        path.to_string_lossy().to_string(),
                        CommandType::External,
                    ))
                }),
        );

    if all {
        commands
            .map(|(filename, path, cmd_type)| entry(filename, path, cmd_type, Span::new(0, 0)))
            .collect()
    } else {
        commands
            .unique_by(|x| x.0.clone())
            .map(|(filename, path, cmd_type)| entry(filename, path, cmd_type, Span::new(0, 0)))
            .collect()
    }
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
            if let Some(entry) = get_entry_in_commands(engine_state, &prog_name, application.span) {
                output.push(entry);
            }
            output.extend(get_all_entries_in_path(
                &prog_name,
                application.span,
                cwd,
                paths,
            ));
            output
        }
        (false, true) => get_first_entry_in_path(&prog_name, application.span, cwd, paths)
            .into_iter()
            .collect(),
        (false, false) => get_entry_in_commands(engine_state, &prog_name, application.span)
            .or_else(|| get_first_entry_in_path(&prog_name, application.span, cwd, paths))
            .into_iter()
            .collect(),
    }
}

fn which(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<PipelineData, ShellError> {
    let head = call.head;
    let which_args = WhichArgs {
        applications: call.rest(engine_state, stack, 0)?,
        all: call.has_flag(engine_state, stack, "all")?,
    };

    let mut output = vec![];

    #[allow(deprecated)]
    let cwd = env::current_dir_str(engine_state, stack)?;
    let paths = env::path_str(engine_state, stack, head)?;

    if which_args.applications.is_empty() {
        return Ok(list_all_executables(engine_state, paths, which_args.all)
            .into_iter()
            .into_pipeline_data(head, engine_state.signals().clone()));
    }

    for app in which_args.applications {
        let values = which_single(
            app,
            which_args.all,
            engine_state,
            cwd.clone(),
            paths.clone(),
        );
        output.extend(values);
    }

    Ok(output
        .into_iter()
        .into_pipeline_data(head, engine_state.signals().clone()))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        crate::test_examples(Which)
    }
}
