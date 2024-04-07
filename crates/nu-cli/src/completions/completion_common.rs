use crate::completions::{matches, CompletionOptions};
use nu_ansi_term::Style;
use nu_engine::env_to_string;
use nu_path::home_dir;
use nu_protocol::engine::{EngineState, Stack};
use nu_protocol::{engine::StateWorkingSet, Span};
use nu_utils::get_ls_colors;
use std::ffi::OsStr;
use std::path::{
    is_separator, Component, Path, PathBuf, MAIN_SEPARATOR as SEP, MAIN_SEPARATOR_STR,
};

fn complete_rec(
    partial: &[String],
    built: &[String],
    cwd: &Path,
    options: &CompletionOptions,
    dir: bool,
    isdir: bool,
) -> Vec<Vec<String>> {
    let mut completions = vec![];

    let mut built_path = cwd.to_path_buf();
    for part in built {
        built_path.push(part);
    }

    if partial.first().is_some_and(|s| s == "..") {
        let mut built = built.to_vec();
        built.push("..".to_string());
        return complete_rec(&partial[1..], &built, cwd, options, dir, isdir);
    }

    let Ok(result) = built_path.read_dir() else {
        return completions;
    };

    for entry in result.filter_map(|e| e.ok()) {
        let entry_name = entry.file_name().to_string_lossy().into_owned();
        let path = entry.path();
        let mut built = built.to_vec();
        built.push(entry_name.clone());

        if !dir || path.is_dir() {
            match partial.split_first() {
                Some((base, rest)) => {
                    if matches(base, &entry_name, options) {
                        if !rest.is_empty() || isdir {
                            completions
                                .extend(complete_rec(rest, &built, cwd, options, dir, isdir));
                            if entry_name.eq(base) {
                                break;
                            }
                        } else {
                            completions.push(built);
                        }
                    }
                }
                None => {
                    completions.push(built);
                }
            }
        }
    }
    completions
}

#[derive(Debug)]
enum OriginalCwd {
    None,
    Home,
    Prefix(String),
    // referencing a single local file
    Local,
}

impl OriginalCwd {
    fn apply(&self, mut p: Vec<String>) -> String {
        match self {
            Self::None => {}
            Self::Home => p.insert(0, "~".to_string()),
            Self::Prefix(s) => p.insert(0, s.clone()),
            Self::Local => p.insert(0, ".".to_string()),
        };

        let mut ret = p.join(MAIN_SEPARATOR_STR);
        if Path::new(&ret).is_dir() {
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

    match partial.rsplit_once(is_separator) {
        // last component after separator ends with '.'
        // it cannot be a CurDir component because this is a partial path by definition
        Some((_, last)) => {
            // If it's just a dot, components() nerfs it altogether
            if last == "." {
                components_vec.push(Component::Normal(OsStr::new(last)));
            } else if last.ends_with('.') {
                components_vec.pop();
                components_vec.push(Component::Normal(OsStr::new(last)));
            }
        }
        // the partial itself is one component ending with a '.'
        None if partial.ends_with('.') => {
            components_vec.pop();
            components_vec.push(Component::Normal(OsStr::new(&partial)));
        }
        _ => {}
    }

    let mut components = components_vec.into_iter().peekable();

    let cwd = match components.peek().cloned() {
        Some(c @ Component::Prefix(..)) => {
            // windows only by definition
            components.next();
            if let Some(Component::RootDir) = components.peek().cloned() {
                components.next();
            };
            original_cwd = OriginalCwd::Prefix(c.as_os_str().to_string_lossy().into_owned());
            [c, Component::RootDir].iter().collect()
        }
        Some(c @ Component::RootDir) => {
            components.next();
            // This is kind of a hack. When joining an empty string with the rest,
            // we add the slash automagically
            original_cwd = OriginalCwd::Prefix(String::new());
            PathBuf::from(c.as_os_str())
        }
        Some(Component::Normal(home)) if home.to_string_lossy() == "~" => {
            components.next();
            original_cwd = OriginalCwd::Home;
            home_dir().unwrap_or(cwd_pathbuf)
        }
        Some(Component::CurDir) => {
            components.next();
            original_cwd = OriginalCwd::Local;
            cwd_pathbuf
        }
        _ => cwd_pathbuf,
    };

    let mut partial = vec![];

    for component in components {
        match component {
            Component::Prefix(..) | Component::RootDir => unreachable!(),
            Component::ParentDir => {
                partial.push("..".to_string());
            }
            Component::Normal(c) => partial.push(c.to_string_lossy().into_owned()),
            _ => {}
        }
    }

    complete_rec(
        partial.as_slice(),
        &[],
        &cwd,
        options,
        want_directory,
        isdir,
    )
    .into_iter()
    .map(|p| {
        let path = original_cwd.apply(p);
        let style = ls_colors.as_ref().map(|lsc| {
            lsc.style_for_path_with_metadata(&path, std::fs::symlink_metadata(&path).ok().as_ref())
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
