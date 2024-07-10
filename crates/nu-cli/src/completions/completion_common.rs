use crate::{
    completions::{matches, CompletionOptions},
    SemanticSuggestion,
};
use nu_ansi_term::Style;
use nu_engine::env_to_string;
use nu_path::{expand_to_real_path, home_dir};
use nu_protocol::{
    engine::{EngineState, Stack, StateWorkingSet},
    levenshtein_distance, Span,
};
use nu_utils::get_ls_colors;
use std::path::{
    is_separator, Component, Path, PathBuf, MAIN_SEPARATOR as SEP, MAIN_SEPARATOR_STR,
};

use super::SortBy;

#[derive(Clone, Default)]
pub struct PathBuiltFromString {
    parts: Vec<String>,
    isdir: bool,
}

fn complete_rec(
    partial: &[&str],
    built: &PathBuiltFromString,
    cwd: &Path,
    options: &CompletionOptions,
    dir: bool,
    isdir: bool,
) -> Vec<PathBuiltFromString> {
    let mut completions = vec![];

    if let Some((&base, rest)) = partial.split_first() {
        if (base == "." || base == "..") && (isdir || !rest.is_empty()) {
            let mut built = built.clone();
            built.parts.push(base.to_string());
            built.isdir = true;
            return complete_rec(rest, &built, cwd, options, dir, isdir);
        }
    }

    let mut built_path = cwd.to_path_buf();
    for part in &built.parts {
        built_path.push(part);
    }

    let Ok(result) = built_path.read_dir() else {
        return completions;
    };

    let mut entries = Vec::new();
    for entry in result.filter_map(|e| e.ok()) {
        let entry_name = entry.file_name().to_string_lossy().into_owned();
        let entry_isdir = entry.path().is_dir();
        let mut built = built.clone();
        built.parts.push(entry_name.clone());
        built.isdir = entry_isdir;

        if !dir || entry_isdir {
            entries.push((entry_name, built));
        }
    }

    let prefix = partial.first().unwrap_or(&"");
    let sorted_entries = sort_completions(prefix, entries, SortBy::Ascending, |(entry, _)| entry);

    for (entry_name, built) in sorted_entries {
        match partial.split_first() {
            Some((base, rest)) => {
                if matches(base, &entry_name, options) {
                    if !rest.is_empty() || isdir {
                        completions.extend(complete_rec(rest, &built, cwd, options, dir, isdir));
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
    completions
}

#[derive(Debug)]
enum OriginalCwd {
    None,
    Home,
    Prefix(String),
}

impl OriginalCwd {
    fn apply(&self, mut p: PathBuiltFromString) -> String {
        match self {
            Self::None => {}
            Self::Home => p.parts.insert(0, "~".to_string()),
            Self::Prefix(s) => p.parts.insert(0, s.clone()),
        };

        let mut ret = p.parts.join(MAIN_SEPARATOR_STR);
        if p.isdir {
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

    let mut cwd = cwd_pathbuf.clone();
    let mut prefix_len = 0;
    let mut original_cwd = OriginalCwd::None;

    let mut components = Path::new(&partial).components().peekable();
    match components.peek().cloned() {
        Some(c @ Component::Prefix(..)) => {
            // windows only by definition
            components.next();
            if let Some(Component::RootDir) = components.peek().cloned() {
                components.next();
            };
            cwd = [c, Component::RootDir].iter().collect();
            prefix_len = c.as_os_str().len();
            original_cwd = OriginalCwd::Prefix(c.as_os_str().to_string_lossy().into_owned());
        }
        Some(c @ Component::RootDir) => {
            components.next();
            // This is kind of a hack. When joining an empty string with the rest,
            // we add the slash automagically
            cwd = PathBuf::from(c.as_os_str());
            prefix_len = 1;
            original_cwd = OriginalCwd::Prefix(String::new());
        }
        Some(Component::Normal(home)) if home.to_string_lossy() == "~" => {
            components.next();
            cwd = home_dir().unwrap_or(cwd_pathbuf);
            prefix_len = 1;
            original_cwd = OriginalCwd::Home;
        }
        _ => {}
    };

    let after_prefix = &partial[prefix_len..];
    let partial: Vec<_> = after_prefix
        .strip_prefix(is_separator)
        .unwrap_or(after_prefix)
        .split(is_separator)
        .filter(|s| !s.is_empty())
        .collect();

    complete_rec(
        partial.as_slice(),
        &PathBuiltFromString::default(),
        &cwd,
        options,
        want_directory,
        isdir,
    )
    .into_iter()
    .map(|p| {
        let path = original_cwd.apply(p);
        let style = ls_colors.as_ref().map(|lsc| {
            lsc.style_for_path_with_metadata(
                &path,
                std::fs::symlink_metadata(expand_to_real_path(&path))
                    .ok()
                    .as_ref(),
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

/// Convenience function to sort suggestions using [`sort_completions`]
pub fn sort_suggestions(
    prefix: &str,
    items: Vec<SemanticSuggestion>,
    sort_by: SortBy,
) -> Vec<SemanticSuggestion> {
    sort_completions(prefix, items, sort_by, |it| &it.suggestion.value)
}

/// # Arguments
/// * `prefix` - What the user's typed, for sorting by Levenshtein distance
pub fn sort_completions<T>(
    prefix: &str,
    mut items: Vec<T>,
    sort_by: SortBy,
    get_value: fn(&T) -> &str,
) -> Vec<T> {
    // Sort items
    match sort_by {
        SortBy::LevenshteinDistance => {
            items.sort_by(|a, b| {
                let a_distance = levenshtein_distance(prefix, get_value(a));
                let b_distance = levenshtein_distance(prefix, get_value(b));
                a_distance.cmp(&b_distance)
            });
        }
        SortBy::Ascending => {
            items.sort_by(|a, b| get_value(a).cmp(get_value(b)));
        }
        SortBy::None => {}
    };

    items
}
