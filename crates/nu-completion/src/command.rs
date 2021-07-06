use super::matchers::Matcher;
use crate::{Completer, CompletionContext, Suggestion};
use indexmap::set::IndexSet;
#[cfg(not(target_arch = "wasm32"))]
use is_executable::IsExecutable;
use nu_test_support::NATIVE_PATH_ENV_VAR;
use std::iter::FromIterator;
use std::path::Path;

pub struct CommandCompleter;

impl<Context> Completer<Context> for CommandCompleter
where
    Context: CompletionContext,
{
    fn complete(&self, ctx: &Context, partial: &str, matcher: &dyn Matcher) -> Vec<Suggestion> {
        let registry = ctx.signature_registry();
        let mut commands: IndexSet<String> = IndexSet::from_iter(registry.names());

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

        if !partial.is_empty() {
            let path_completer = crate::path::PathCompleter;
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

#[cfg(not(target_arch = "wasm32"))]
fn is_executable(path: &Path) -> bool {
    // This call to a crate essentially checks the PATHEXT on Windows and does some
    // low level WinAPI calls to determine if the file is executable. It seems quite
    // a bit faster than calling path.metadata().
    // On Unix, this checks the file metadata. The underlying code traverses symlinks.
    path.is_executable()
}

#[cfg(target_arch = "wasm32")]
fn is_executable(_path: &Path) -> bool {
    false
}

// TODO cache these, but watch for changes to PATH
fn find_path_executables() -> Option<IndexSet<String>> {
    let path_var = std::env::var_os(NATIVE_PATH_ENV_VAR)?;
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
