use std::fs::Metadata;
use std::iter::FromIterator;

use indexmap::set::IndexSet;

use crate::completion::{Context, Suggestion};
use crate::context;

pub struct Completer;

impl Completer {
    pub fn complete(&self, ctx: &Context<'_>, partial: &str) -> Vec<Suggestion> {
        let context: &context::Context = ctx.as_ref();
        let mut commands: IndexSet<String> = IndexSet::from_iter(context.registry.names());

        // Command suggestions can come from three possible sets:
        //   1. internal command names,
        //   2. external command names relative to PATH env var, and
        //   3. any other executable (that matches what's been typed so far).

        let path_executables = find_path_executables().unwrap_or_default();

        // TODO quote these, if necessary
        commands.extend(path_executables.into_iter());

        let mut suggestions: Vec<_> = commands
            .into_iter()
            .filter(|v| v.starts_with(partial))
            .map(|v| Suggestion {
                replacement: format!("{} ", v),
                display: v,
            })
            .collect();

        if partial != "" {
            let path_completer = crate::completion::path::Completer::new();
            let path_results = path_completer.complete(ctx, partial);
            suggestions.extend(path_results.into_iter().filter(|suggestion| {
                std::fs::metadata(&suggestion.replacement)
                    .ok()
                    .map(|metadata| metadata.is_dir() || is_executable(metadata))
                    .unwrap_or(false)
            }));
        }

        suggestions
    }
}

#[cfg(windows)]
fn pathext() -> Option<Vec<String>> {
    std::env::var_os("PATHEXT").map(|v| {
        v.to_string_lossy()
            .split(';')
            // Cut off the leading '.' character
            .map(|ext| ext[1..].to_string())
            .collect::<Vec<_>>()
    })
}

#[cfg(windows)]
fn is_executable(metadata: Metadata) -> bool {
    let file_type = metadata.file_type();

    // If the entry isn't a file, it cannot be executable
    if !(file_type.is_file() || file_type.is_symlink()) {
        return false;
    }

    if let Some(extension) = file.path().extension() {
        if let Some(exts) = pathext() {
            exts.iter()
                .any(|ext| extension.to_string_lossy().eq_ignore_ascii_case(ext))
        } else {
            false
        }
    } else {
        false
    }
}

#[cfg(target_arch = "wasm32")]
fn is_executable(_metadata: Metadata) -> bool {
    false
}

#[cfg(unix)]
fn is_executable(metadata: Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;

    let filetype = metadata.file_type();
    let permissions = metadata.permissions();

    // The file is executable if it is a directory or a symlink and the permissions are set for
    // owner, group, or other
    (filetype.is_file() || filetype.is_symlink()) && (permissions.mode() & 0o111 != 0)
}

// TODO cache these, but watch for changes to PATH
fn find_path_executables() -> Option<IndexSet<String>> {
    let path_var = std::env::var_os("PATH")?;
    let paths: Vec<_> = std::env::split_paths(&path_var).collect();

    let mut executables: IndexSet<String> = IndexSet::new();
    for path in paths {
        if let Ok(mut contents) = std::fs::read_dir(path) {
            while let Some(Ok(item)) = contents.next() {
                if let Ok(metadata) = item.metadata() {
                    if is_executable(metadata) {
                        if let Ok(name) = item.file_name().into_string() {
                            executables.insert(name);
                        }
                    }
                }
            }
        }
    }

    Some(executables)
}
