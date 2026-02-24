use nu_engine::{command_prelude::*, env};
use nu_protocol::engine::CommandType;
use std::collections::HashSet;
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
            .switch("all", "List all executables.", Some('a'))
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

/// Returns the source file path that covers `span`, if any.
fn file_for_span(engine_state: &EngineState, span: Span) -> Option<String> {
    engine_state
        .files()
        .find(|f| f.covered_span.contains_span(span))
        .map(|f| f.name.to_string())
}

/// Returns the source file path for a declaration, if it can be determined.
///
/// - Aliases: resolved via `decl_span()` (the alias expansion span)
/// - Custom commands: resolved from the block's span via `block_id()`
/// - Plugins: resolved from the plugin identity's filename
/// - Known externals (`extern` declarations): resolved via `decl_span()`
fn file_for_decl(
    engine_state: &EngineState,
    decl: &dyn nu_protocol::engine::Command,
) -> Option<String> {
    if let Some(block_id) = decl.block_id() {
        return engine_state
            .get_block(block_id)
            .span
            .and_then(|sp| file_for_span(engine_state, sp));
    }
    #[cfg(feature = "plugin")]
    if decl.is_plugin() {
        return decl
            .plugin_identity()
            .map(|id| id.filename().to_string_lossy().to_string());
    }
    if let Some(span) = decl.decl_span() {
        return file_for_span(engine_state, span);
    }
    None
}

// Shortcut for creating an entry to the output table.
fn entry(
    arg: impl Into<String>,
    path: impl Into<String>,
    cmd_type: CommandType,
    definition: Option<String>,
    file: Option<String>,
    span: Span,
) -> Value {
    let arg = arg.into();
    let path = path.into();
    let path_value = if path.is_empty() {
        file.unwrap_or_default()
    } else {
        path.clone()
    };

    let mut record = record! {
        "command" => Value::string(arg, span),
        "path" => Value::string(path_value, span),
        "type" => Value::string(cmd_type.to_string(), span),
    };

    if let Some(def) = definition {
        record.insert("definition", Value::string(def, span));
    }

    Value::record(record, span)
}

fn get_entry_in_commands(engine_state: &EngineState, name: &str, span: Span) -> Option<Value> {
    let decl_id = engine_state.find_decl(name.as_bytes(), &[])?;
    let decl = engine_state.get_decl(decl_id);
    let definition = if decl.command_type() == CommandType::Alias {
        decl.as_alias().map(|alias| {
            String::from_utf8_lossy(engine_state.get_span_contents(alias.wrapped_call.span))
                .to_string()
        })
    } else {
        None
    };
    let file = file_for_decl(engine_state, decl);
    Some(entry(name, "", decl.command_type(), definition, file, span))
}

fn get_first_entry_in_path(
    item: &str,
    span: Span,
    cwd: impl AsRef<Path>,
    paths: impl AsRef<OsStr>,
) -> Option<Value> {
    which::which_in(item, Some(paths), cwd)
        .map(|path| {
            let full_path = path.to_string_lossy().to_string();
            entry(
                item,
                full_path.clone(),
                CommandType::External,
                None,
                Some(full_path),
                span,
            )
        })
        .ok()
}

fn get_all_entries_in_path(
    item: &str,
    span: Span,
    cwd: impl AsRef<Path>,
    paths: impl AsRef<OsStr>,
) -> Vec<Value> {
    // `which_in_all` canonicalizes every result path. On systems where PATH
    // contains both a real directory and a symlink pointing to the same place
    // (e.g. `/usr/bin` and `/bin -> /usr/bin` on WSL/Debian), the same
    // canonical path would appear multiple times. The HashSet deduplicates
    // those before we build the output rows.
    let mut seen = HashSet::new();
    which::which_in_all(item, Some(paths), cwd)
        .map(|iter| {
            iter.filter(|path| seen.insert(path.clone()))
                .map(|path| {
                    let full_path = path.to_string_lossy().to_string();
                    entry(
                        item,
                        full_path.clone(),
                        CommandType::External,
                        None,
                        Some(full_path),
                        span,
                    )
                })
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

    let mut results = Vec::with_capacity(decls.len());
    let mut seen_commands = HashSet::with_capacity(decls.len());

    for (name_bytes, decl_id) in decls {
        let name = String::from_utf8_lossy(&name_bytes).to_string();
        seen_commands.insert(name.clone());
        let decl = engine_state.get_decl(decl_id);
        let definition = if decl.command_type() == CommandType::Alias {
            decl.as_alias().map(|alias| {
                String::from_utf8_lossy(engine_state.get_span_contents(alias.wrapped_call.span))
                    .to_string()
            })
        } else {
            None
        };
        let file = file_for_decl(engine_state, decl);

        results.push(entry(
            name,
            String::new(),
            decl.command_type(),
            definition,
            file,
            Span::unknown(),
        ));
    }

    // Add PATH executables
    let path_iter = sys::RealSys
        .env_split_paths(paths.as_ref())
        .into_iter()
        .filter_map(|dir| fs::read_dir(dir).ok())
        .flat_map(|entries| entries.flatten())
        .map(|entry| entry.path())
        .filter_map(|path| {
            if !path.is_executable() {
                return None;
            }
            let filename = path.file_name()?.to_string_lossy().to_string();

            if !all && !seen_commands.insert(filename.clone()) {
                return None;
            }

            let full_path = path.to_string_lossy().to_string();
            Some(entry(
                filename,
                full_path.clone(),
                CommandType::External,
                None,
                Some(full_path),
                Span::unknown(),
            ))
        });

    results.extend(path_iter);
    results
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
    let cwd = cwd.as_ref();
    let paths = paths.as_ref();
    let (external, prog_name) = if application.item.starts_with('^') {
        (true, application.item[1..].to_string())
    } else {
        (false, application.item.clone())
    };

    // If prog_name is an external command, don't search for nu-specific programs.
    // If all is false, we can save some time by only searching for the first match.
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

    let cwd = engine_state.cwd_as_string(Some(stack))?;

    // PATH may not be set in minimal environments (e.g. plugin test harnesses).
    // In that case we can still resolve built-ins, aliases, custom commands and
    // known externals; we just won't find any PATH-based binaries.
    let paths = env::path_str(engine_state, stack, head).unwrap_or_default();

    if which_args.applications.is_empty() {
        return Ok(list_all_executables(engine_state, &paths, which_args.all)
            .into_iter()
            .into_pipeline_data(head, engine_state.signals().clone()));
    }

    for app in which_args.applications {
        let values = which_single(app, which_args.all, engine_state, &cwd, &paths);
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

// --------------------
// Copied from https://docs.rs/is_executable/ v1.0.5
// Removed path.exists() check in `mod windows`.

/// An extension trait for `std::fs::Path` providing an `is_executable` method.
///
/// See the module documentation for examples.
pub trait IsExecutable {
    /// Returns `true` if there is a file at the given path and it is
    /// executable. Returns `false` otherwise.
    ///
    /// See the module documentation for details.
    fn is_executable(&self) -> bool;
}

#[cfg(unix)]
mod unix {
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;

    use super::IsExecutable;

    impl IsExecutable for Path {
        fn is_executable(&self) -> bool {
            let metadata = match self.metadata() {
                Ok(metadata) => metadata,
                Err(_) => return false,
            };
            let permissions = metadata.permissions();
            metadata.is_file() && permissions.mode() & 0o111 != 0
        }
    }
}

#[cfg(target_os = "windows")]
mod windows {
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;

    use windows::Win32::Storage::FileSystem::GetBinaryTypeW;
    use windows::core::PCWSTR;

    use super::IsExecutable;

    impl IsExecutable for Path {
        fn is_executable(&self) -> bool {
            // Check using file extension
            if let Some(pathext) = std::env::var_os("PATHEXT")
                && let Some(extension) = self.extension()
            {
                let extension = extension.to_string_lossy();

                // Originally taken from:
                // https://github.com/nushell/nushell/blob/93e8f6c05e1e1187d5b674d6b633deb839c84899/crates/nu-cli/src/completion/command.rs#L64-L74
                return pathext
                    .to_string_lossy()
                    .split(';')
                    // Filter out empty tokens and ';' at the end
                    .filter(|f| f.len() > 1)
                    .any(|ext| {
                        // Cut off the leading '.' character
                        let ext = &ext[1..];
                        extension.eq_ignore_ascii_case(ext)
                    });
            }

            // Check using file properties
            // This code is only reached if there is no file extension or retrieving PATHEXT fails
            let windows_string: Vec<u16> = self.as_os_str().encode_wide().chain(Some(0)).collect();
            let mut binary_type: u32 = 0;

            let result =
                unsafe { GetBinaryTypeW(PCWSTR(windows_string.as_ptr()), &mut binary_type) };
            if result.is_ok()
                && let 0..=6 = binary_type
            {
                return true;
            }

            false
        }
    }
}

// For WASI, we can't check if a file is executable
// Since wasm and wasi
//  is not supposed to add executables ideologically,
// specify them collectively
#[cfg(any(target_os = "wasi", target_family = "wasm"))]
mod wasm {
    use std::path::Path;

    use super::IsExecutable;

    impl IsExecutable for Path {
        fn is_executable(&self) -> bool {
            false
        }
    }
}
