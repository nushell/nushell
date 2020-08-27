use std::fs::{read_dir, DirEntry};
use std::iter::FromIterator;

#[cfg(all(windows, feature = "ichwh"))]
use ichwh::{IchwhError, IchwhResult};
use indexmap::set::IndexSet;

use crate::completion::matchers::Matcher;
use crate::completion::{Context, Suggestion};
use crate::context;

pub struct Completer;

impl Completer {
    pub fn complete(
        &self,
        ctx: &Context<'_>,
        partial: &str,
        matcher: &Box<dyn Matcher>,
    ) -> Vec<Suggestion> {
        let context: &context::Context = ctx.as_ref();
        let mut commands: IndexSet<String> = IndexSet::from_iter(context.registry.names());

        let path_executables = find_path_executables().unwrap_or_default();

        // TODO quote these, if necessary
        commands.extend(path_executables.into_iter());

        commands
            .into_iter()
            .filter(|v| matcher.matches(partial, v))
            .map(|v| Suggestion {
                replacement: format!("{} ", v),
                display: v,
            })
            .collect()
    }
}

// These is_executable/pathext implementations are copied from ichwh and modified
// to not be async

#[cfg(windows)]
fn pathext() -> IchwhResult<Vec<String>> {
    Ok(std::env::var_os("PATHEXT")
        .ok_or(IchwhError::PathextNotDefined)?
        .to_string_lossy()
        .split(';')
        // Cut off the leading '.' character
        .map(|ext| ext[1..].to_string())
        .collect::<Vec<_>>())
}

#[cfg(windows)]
fn is_executable(file: &DirEntry) -> bool {
    if let Ok(metadata) = file.metadata() {
        let file_type = metadata.file_type();

        // If the entry isn't a file, it cannot be executable
        if !(file_type.is_file() || file_type.is_symlink()) {
            return false;
        }

        if let Some(extension) = file.path().extension() {
            if let Ok(exts) = pathext() {
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
fn is_executable(_file: &DirEntry) -> bool {
    false
}

#[cfg(unix)]
fn is_executable(file: &DirEntry) -> bool {
    use std::os::unix::fs::PermissionsExt;

    let metadata = file.metadata();

    if let Ok(metadata) = metadata {
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
        if let Ok(mut contents) = read_dir(path) {
            while let Some(Ok(item)) = contents.next() {
                if is_executable(&item) {
                    if let Ok(name) = item.file_name().into_string() {
                        executables.insert(name);
                    }
                }
            }
        }
    }

    Some(executables)
}
