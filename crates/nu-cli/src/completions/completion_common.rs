use crate::completions::{matches, CompletionOptions};
use nu_path::home_dir;
use std::path::{is_separator, Component, Path, PathBuf, MAIN_SEPARATOR as SEP};

fn complete_rec(
    partial: &[String],
    cwd: &Path,
    options: &CompletionOptions,
    dir: bool,
    isdir: bool,
) -> Vec<PathBuf> {
    let mut completions = vec![];

    if let Ok(result) = cwd.read_dir() {
        for entry in result.filter_map(|e| e.ok()) {
            let entry_name = entry.file_name().to_string_lossy().into_owned();
            let path = entry.path();

            if !dir || path.is_dir() {
                match partial.first() {
                    Some(base) if matches(base, &entry_name, options) => {
                        let partial = &partial[1..];
                        if !partial.is_empty() || isdir {
                            completions.extend(complete_rec(partial, &path, options, dir, isdir))
                        } else {
                            completions.push(path)
                        }
                    }
                    None => completions.push(path),
                    _ => {}
                }
            }
        }
    }
    completions
}

enum OriginalCwd {
    None,
    Home(PathBuf),
    Some(PathBuf),
}

impl OriginalCwd {
    fn apply(&self, p: &Path) -> String {
        let mut ret = match self {
            Self::None => p.to_string_lossy().into_owned(),
            Self::Some(base) => pathdiff::diff_paths(p, base)
                .unwrap_or(p.to_path_buf())
                .to_string_lossy()
                .into_owned(),
            Self::Home(home) => match p.strip_prefix(home) {
                Ok(suffix) => format!("~{}{}", SEP, suffix.to_string_lossy()),
                _ => p.to_string_lossy().into_owned(),
            },
        };

        if p.is_dir() {
            ret.push(SEP);
        }
        ret
    }
}

fn surround_remove(partial: &str) -> &str {
    for c in ['`', '"', '\''] {
        if partial.starts_with(c) {
            let ret = partial.strip_prefix(c).unwrap_or(partial);
            return if partial.ends_with(c) {
                ret.strip_suffix(c).unwrap_or(ret)
            } else {
                ret
            };
        }
    }
    partial
}

pub fn complete_item(
    want_directory: bool,
    span: nu_protocol::Span,
    partial: &str,
    cwd: &str,
    options: &CompletionOptions,
) -> Vec<(nu_protocol::Span, String)> {
    let partial = surround_remove(partial);
    let isdir = partial.ends_with(is_separator);
    let cwd_pathbuf = Path::new(cwd).to_path_buf();
    let mut original_cwd = OriginalCwd::None;
    let mut components = Path::new(partial).components().peekable();
    let mut cwd = match components.peek().cloned() {
        Some(c @ Component::Prefix(..)) => {
            // windows only by definition
            components.next();
            if let Some(Component::RootDir) = components.peek().cloned() {
                components.next();
            };
            [c, Component::RootDir].iter().collect()
        }
        Some(c @ Component::RootDir) => {
            components.next();
            PathBuf::from(c.as_os_str())
        }
        Some(Component::Normal(home)) if home.to_string_lossy() == "~" => {
            components.next();
            original_cwd = OriginalCwd::Home(home_dir().unwrap_or(cwd_pathbuf.clone()));
            home_dir().unwrap_or(cwd_pathbuf)
        }
        _ => {
            original_cwd = OriginalCwd::Some(cwd_pathbuf.clone());
            cwd_pathbuf
        }
    };

    let mut partial = vec![];

    for component in components {
        match component {
            Component::Prefix(..) => unreachable!(),
            Component::RootDir => unreachable!(),
            Component::CurDir => {}
            Component::ParentDir => {
                if partial.pop().is_none() {
                    cwd.pop();
                }
            }
            Component::Normal(c) => {
                partial.push(c.to_string_lossy().into_owned());
            }
        }
    }

    complete_rec(partial.as_slice(), &cwd, options, want_directory, isdir)
        .into_iter()
        .map(|p| (span, escape_path(original_cwd.apply(&p), want_directory)))
        .collect()
}

// Fix files or folders with quotes or hashes
pub fn escape_path(path: String, dir: bool) -> String {
    let filename_contaminated = !dir
        && path.contains([
            '\'', '"', ' ', '#', '(', ')', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9',
        ]);
    let dirname_contaminated = dir && path.contains(['\'', '"', ' ', '#']);
    if filename_contaminated || dirname_contaminated {
        format!("`{path}`")
    } else {
        path
    }
}
