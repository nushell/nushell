use crate::completions::{matches, CompletionOptions};
use nu_path::home_dir;
use nu_protocol::{engine::StateWorkingSet, Span};
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
                            completions.extend(complete_rec(partial, &path, options, dir, isdir));
                            if entry_name.eq(base) {
                                break;
                            }
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
    // referencing a single local file
    Local(PathBuf),
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
            Self::Local(base) => Path::new(".")
                .join(pathdiff::diff_paths(p, base).unwrap_or(p.to_path_buf()))
                .to_string_lossy()
                .into_owned(),
        };

        if p.is_dir() {
            ret.push(SEP);
        }
        ret
    }
}

fn surround_remove(partial: &str) -> String {
    for c in ['`', '"', '\''] {
        if partial.starts_with(c) {
            let ret = partial.strip_prefix(c).unwrap_or(partial);
            return match ret.split(c).collect::<Vec<_>>()[..] {
                [inside] => inside.to_string(),
                [inside, outside] if inside.ends_with(is_separator) => format!("{inside}{outside}"),
                _ => ret.to_string(),
            };
        }
    }
    partial.to_string()
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
    let mut components = Path::new(&partial).components().peekable();
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
        Some(Component::CurDir) => {
            components.next();
            original_cwd = match components.peek().cloned() {
                Some(Component::Normal(_)) | None => OriginalCwd::Local(cwd_pathbuf.clone()),
                _ => OriginalCwd::Some(cwd_pathbuf.clone()),
            };
            cwd_pathbuf
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
            Component::Normal(c) => partial.push(c.to_string_lossy().into_owned()),
        }
    }

    complete_rec(partial.as_slice(), &cwd, options, want_directory, isdir)
        .into_iter()
        .map(|p| (span, escape_path(original_cwd.apply(&p), want_directory)))
        .collect()
}

// Fix files or folders with quotes or hashes
pub fn escape_path(path: String, dir: bool) -> String {
    let filename_contaminated = !dir && path.contains(['\'', '"', ' ', '#', '(', ')']);
    let dirname_contaminated = dir && path.contains(['\'', '"', ' ', '#']);
    let maybe_flag = path.starts_with('-');
    let maybe_number = path.parse::<f64>().is_ok();
    if filename_contaminated || dirname_contaminated || maybe_flag || maybe_number {
        format!("`{path}`")
    } else {
        path
    }
}

pub struct AdjustView {
    pub prefix: String,
    pub span: Span,
    pub readjusted: bool,
}

pub fn adjust_if_intermediate(
    prefix: &[u8],
    working_set: &StateWorkingSet,
    mut span: nu_protocol::Span,
) -> AdjustView {
    let span_contents = String::from_utf8_lossy(working_set.get_span_contents(span)).to_string();
    let mut prefix = String::from_utf8_lossy(prefix).to_string();

    // A difference of 1 because of the cursor's unicode code point in between.
    // Using .chars().count() because unicode and Windows.
    let readjusted = span_contents.chars().count() - prefix.chars().count() > 1;
    if readjusted {
        let remnant: String = span_contents
            .chars()
            .skip(prefix.chars().count() + 1)
            .take_while(|&c| !is_separator(c))
            .collect();
        prefix.push_str(&remnant);
        span = Span::new(span.start, span.start + prefix.chars().count() + 1);
    }
    AdjustView {
        prefix,
        span,
        readjusted,
    }
}
