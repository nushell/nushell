use crate::completions::{matches, CompletionOptions};
use nu_ansi_term::Style;
use nu_engine::env_to_string;
use nu_path::home_dir;
use nu_protocol::{
    engine::{EngineState, Stack, StateWorkingSet},
    Span,
};
use nu_utils::get_ls_colors;
use std::{
    ffi::OsStr,
    path::{is_separator, Component, Path, PathBuf, MAIN_SEPARATOR as SEP},
};

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
    engine_state: &EngineState,
    stack: &Stack,
) -> Vec<(nu_protocol::Span, String, Option<Style>)> {
    let partial = surround_remove(partial);
    let isdir = partial.ends_with(is_separator);
    let cwd_pathbuf = Path::new(cwd).to_path_buf();
    let ls_colors = (engine_state.config.use_ls_colors_completions
        && engine_state.config.use_ansi_coloring)
        .then(|| {
            let ls_colors_env_str = match stack.get_env_var(engine_state, "LS_COLORS") {
                Some(v) => env_to_string("LS_COLORS", &v, engine_state, stack).ok(),
                None => None,
            };
            get_ls_colors(ls_colors_env_str)
        });
    let mut original_cwd = OriginalCwd::None;
    let mut components_vec: Vec<Component> = Path::new(&partial).components().collect();

    // Path components that end with a single "." get normalized away,
    // so if the partial path ends in a literal "." we must add it back in manually
    if partial.ends_with('.') && partial.len() > 1 {
        components_vec.push(Component::Normal(OsStr::new(".")));
    };
    let mut components = components_vec.into_iter().peekable();

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
        .map(|p| {
            let path = original_cwd.apply(&p);
            let style = ls_colors.as_ref().map(|lsc| {
                lsc.style_for_path_with_metadata(
                    &path,
                    std::fs::symlink_metadata(&path).ok().as_ref(),
                )
                .map(lscolors::Style::to_nu_ansi_term_style)
                .unwrap_or_default()
            });
            (span, escape_path(path, want_directory), style)
        })
        .collect()
}

// Fix files or folders with quotes or hashes
pub fn escape_path(path: String, dir: bool) -> String {
    // make glob pattern have the highest priority.
    let glob_contaminated = path.contains(['[', '*', ']', '?']);
    if glob_contaminated {
        return if path.contains('\'') {
            // decide to use double quote, also need to escape `"` in path
            // or else users can't do anything with completed path either.
            format!("\"{}\"", path.replace('"', r#"\""#))
        } else {
            format!("'{path}'")
        };
    }

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
