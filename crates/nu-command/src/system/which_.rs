use nu_engine::{command_prelude::*, env};
use nu_protocol::engine::CommandType;
use regex::Regex;
use std::string::String as DefaultString;
use std::{ffi::OsStr, path::Path};

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
            .switch("regex", "list executables by regex", Some('r'))
            .category(Category::System)
    }

    fn description(&self) -> &str {
        "Finds a program file, alias or custom command."
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

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Find if the 'myapp' application is available",
                example: "which myapp",
                result: None,
            },
            Example {
                description: "Find all python versions",
                example: "which -a -r 'python[0-9]+'",
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

fn get_entries_regex(regex: &Regex, span: Span, return_first_match: bool) -> Vec<Value> {
    let Ok(paths) = which::which_re(regex) else {
        return vec![];
    };

    let matches = paths.filter_map(|path| {
        Path::new(&path)
            .file_name()
            .and_then(OsStr::to_str)
            .map(|filename| {
                entry(
                    filename,
                    path.to_string_lossy(),
                    CommandType::External,
                    span,
                )
            })
    });

    if return_first_match {
        matches.take(1).collect()
    } else {
        matches.collect()
    }
}

fn which_single(
    application: Spanned<String>,
    all: bool,
    regex: bool,
    engine_state: &EngineState,
    cwd: impl AsRef<Path>,
    paths: impl AsRef<OsStr>,
) -> Result<Vec<Value>, ShellError> {
    if regex {
        let regex = Regex::new(&application.item).map_err(|e| ShellError::IncorrectValue {
            msg: e.to_string(),
            val_span: application.span,
            call_span: application.span,
        })?;

        let mut internal_commands: Vec<Value> = engine_state
            .get_decls_sorted(false)
            .iter()
            .map(|x| {
                let decl = engine_state.get_decl(x.1);
                (DefaultString::from_utf8_lossy(&x.0), decl.command_type())
            })
            .filter(|x| regex.is_match(&x.0))
            .map(|x| entry(x.0, "", x.1, application.span))
            .collect();

        if !all && !internal_commands.is_empty() {
            return Ok(internal_commands.into_iter().take(1).collect());
        }

        internal_commands.extend(get_entries_regex(&regex, application.span, !all));

        return Ok(internal_commands);
    }

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
        (true, true) => Ok(get_all_entries_in_path(
            &prog_name,
            application.span,
            cwd,
            paths,
        )),
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
            Ok(output)
        }
        (false, true) => Ok(
            get_first_entry_in_path(&prog_name, application.span, cwd, paths)
                .into_iter()
                .collect(),
        ),
        (false, false) => Ok(
            get_entry_in_commands(engine_state, &prog_name, application.span)
                .or_else(|| get_first_entry_in_path(&prog_name, application.span, cwd, paths))
                .into_iter()
                .collect(),
        ),
    }
}

#[derive(Debug)]
struct WhichArgs {
    applications: Vec<Spanned<String>>,
    all: bool,
    regex: bool,
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
        regex: call.has_flag(engine_state, stack, "regex")?,
    };

    if which_args.applications.is_empty() {
        return Err(ShellError::MissingParameter {
            param_name: "application".into(),
            span: head,
        });
    }

    let mut output = vec![];

    #[allow(deprecated)]
    let cwd = env::current_dir_str(engine_state, stack)?;
    let paths = env::path_str(engine_state, stack, head)?;

    for app in which_args.applications {
        let values = which_single(
            app,
            which_args.all,
            which_args.regex,
            engine_state,
            cwd.clone(),
            paths.clone(),
        )?;
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
