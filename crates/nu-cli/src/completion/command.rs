use std::iter::FromIterator;
use std::path::Path;

use indexmap::set::IndexSet;

use super::matchers::Matcher;
use crate::completion::{Completer, CompletionContext, Suggestion};
use crate::evaluation_context::EvaluationContext;

pub struct CommandCompleter;

impl Completer for CommandCompleter {
    fn complete(
        &self,
        ctx: &CompletionContext<'_>,
        partial: &str,
        matcher: &dyn Matcher,
    ) -> Vec<Suggestion> {
        let context: &EvaluationContext = ctx.as_ref();
        let mut commands: IndexSet<String> = IndexSet::from_iter(context.scope.get_command_names());

        // Command suggestions can come from three possible sets:
        //   1. internal command names,
        //   2. external command names relative to PATH env var, and
        //   3. any other executable (that matches what's been typed so far).

        let path_executables = find_path_executables().unwrap_or_default();

        // TODO quote these, if necessary
        commands.extend(path_executables.into_iter());

        let mut suggestions: Vec<_> = commands
            .into_iter()
            .filter(|v| matcher.matches(partial, v))
            .map(|v| Suggestion {
                replacement: v.clone(),
                display: v,
            })
            .collect();

        if partial != "" {
            let path_completer = crate::completion::path::PathCompleter;
            let path_results = path_completer.path_suggestions(partial, matcher);
            let iter = path_results.into_iter().filter_map(|path_suggestion| {
                let path = path_suggestion.path;
                if path.is_dir() || is_executable(&path) {
                    Some(path_suggestion.suggestion)
                } else {
                    None
                }
            });

            suggestions.extend(iter);
        }

        suggestions
    }
}

// TODO create a struct for "is executable" and store this information in it so we don't recompute
//      on every dir entry

#[cfg(windows)]
fn pathext() -> Option<Vec<String>> {
    std::env::var_os("PATHEXT").map(|v| {
        v.to_string_lossy()
            .split(';')
            // Filter out empty tokens and ';' at the end
            .filter(|f| f.len() > 1)
            // Cut off the leading '.' character
            .map(|ext| ext[1..].to_string())
            .collect::<Vec<_>>()
    })
}

#[cfg(windows)]
fn is_executable(path: &Path) -> bool {
    if let Ok(metadata) = path.metadata() {
        let file_type = metadata.file_type();

        // If the entry isn't a file, it cannot be executable
        if !(file_type.is_file() || file_type.is_symlink()) {
            return false;
        }

        if let Some(extension) = path.extension() {
            if let Some(exts) = pathext() {
                exts.iter()
                    .any(|ext| extension.to_string_lossy().eq_ignore_ascii_case(ext))
            } else {
                false
            }
        } else {
            false
        }
    } else {
        false
    }
}

#[cfg(target_arch = "wasm32")]
fn is_executable(_path: &Path) -> bool {
    false
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;

    if let Ok(metadata) = path.metadata() {
        let filetype = metadata.file_type();
        let permissions = metadata.permissions();

        // The file is executable if it is a directory or a symlink and the permissions are set for
        // owner, group, or other
        (filetype.is_file() || filetype.is_symlink()) && (permissions.mode() & 0o111 != 0)
    } else {
        false
    }
}

// TODO cache these, but watch for changes to PATH
fn find_path_executables() -> Option<IndexSet<String>> {
    let path_var = std::env::var_os("PATH")?;
    let paths: Vec<_> = std::env::split_paths(&path_var).collect();

    let mut executables: IndexSet<String> = IndexSet::new();
    for path in paths {
        if let Ok(mut contents) = std::fs::read_dir(path) {
            while let Some(Ok(item)) = contents.next() {
                if is_executable(&item.path()) {
                    if let Ok(name) = item.file_name().into_string() {
                        executables.insert(name);
                    }
                }
            }
        }
    }

    Some(executables)
}
