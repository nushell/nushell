use super::{completion_options::NuMatcher, MatchAlgorithm};
use crate::completions::CompletionOptions;
use nu_ansi_term::Style;
use nu_engine::env_to_string;
use nu_path::dots::expand_ndots;
use nu_path::{expand_to_real_path, home_dir};
use nu_protocol::{
    engine::{EngineState, Stack, StateWorkingSet},
    Span,
};
use nu_utils::get_ls_colors;
use nu_utils::IgnoreCaseExt;
use std::path::{is_separator, Component, Path, PathBuf, MAIN_SEPARATOR as SEP};

#[derive(Clone, Default)]
pub struct PathBuiltFromString {
    cwd: PathBuf,
    parts: Vec<String>,
    isdir: bool,
}

/// Recursively goes through paths that match a given `partial`.
/// built: State struct for a valid matching path built so far.
///
/// `isdir`: whether the current partial path has a trailing slash.
/// Parsing a path string into a pathbuf loses that bit of information.
///
/// want_directory: Whether we want only directories as completion matches.
/// Some commands like `cd` can only be run on directories whereas others
/// like `ls` can be run on regular files as well.
fn complete_rec(
    partial: &[&str],
    built_paths: &[PathBuiltFromString],
    options: &CompletionOptions,
    want_directory: bool,
    isdir: bool,
) -> Vec<PathBuiltFromString> {
    if let Some((&base, rest)) = partial.split_first() {
        if base.chars().all(|c| c == '.') && (isdir || !rest.is_empty()) {
            let built_paths: Vec<_> = built_paths
                .iter()
                .map(|built| {
                    let mut built = built.clone();
                    built.parts.push(base.to_string());
                    built.isdir = true;
                    built
                })
                .collect();
            return complete_rec(rest, &built_paths, options, want_directory, isdir);
        }
    }

    let prefix = partial.first().unwrap_or(&"");
    let mut matcher = NuMatcher::new(prefix, options.clone());

    for built in built_paths {
        let mut path = built.cwd.clone();
        for part in &built.parts {
            path.push(part);
        }

        let Ok(result) = path.read_dir() else {
            continue;
        };

        for entry in result.filter_map(|e| e.ok()) {
            let entry_name = entry.file_name().to_string_lossy().into_owned();
            let entry_isdir = entry.path().is_dir();
            let mut built = built.clone();
            built.parts.push(entry_name.clone());
            built.isdir = entry_isdir;

            if !want_directory || entry_isdir {
                matcher.add(entry_name.clone(), (entry_name, built));
            }
        }
    }

    let mut completions = vec![];
    for (entry_name, built) in matcher.results() {
        match partial.split_first() {
            Some((base, rest)) => {
                // We use `isdir` to confirm that the current component has
                // at least one next component or a slash.
                // Serves as confirmation to ignore longer completions for
                // components in between.
                if !rest.is_empty() || isdir {
                    completions.extend(complete_rec(
                        rest,
                        &[built],
                        options,
                        want_directory,
                        isdir,
                    ));
                } else {
                    completions.push(built);
                }

                // For https://github.com/nushell/nushell/issues/13204
                if isdir && options.match_algorithm == MatchAlgorithm::Prefix {
                    let exact_match = if options.case_sensitive {
                        entry_name.eq(base)
                    } else {
                        entry_name.to_folded_case().eq(&base.to_folded_case())
                    };
                    if exact_match {
                        break;
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
    fn apply(&self, mut p: PathBuiltFromString, path_separator: char) -> String {
        match self {
            Self::None => {}
            Self::Home => p.parts.insert(0, "~".to_string()),
            Self::Prefix(s) => p.parts.insert(0, s.clone()),
        };

        let mut ret = p.parts.join(&path_separator.to_string());
        if p.isdir {
            ret.push(path_separator);
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

pub struct FileSuggestion {
    pub span: nu_protocol::Span,
    pub path: String,
    pub style: Option<Style>,
    pub cwd: PathBuf,
}

/// # Parameters
/// * `cwds` - A list of directories in which to search. The only reason this isn't a single string
///   is because dotnu_completions searches in multiple directories at once
pub fn complete_item(
    want_directory: bool,
    span: nu_protocol::Span,
    partial: &str,
    cwds: &[impl AsRef<str>],
    options: &CompletionOptions,
    engine_state: &EngineState,
    stack: &Stack,
) -> Vec<FileSuggestion> {
    let cleaned_partial = surround_remove(partial);
    let isdir = cleaned_partial.ends_with(is_separator);
    #[cfg(windows)]
    let cleaned_partial = if let Some(absolute_partial) =
        stack.pwd_per_drive.expand_pwd(Path::new(&cleaned_partial))
    {
        absolute_partial.display().to_string()
    } else {
        cleaned_partial
    };
    let expanded_partial = expand_ndots(Path::new(&cleaned_partial));
    let should_collapse_dots = expanded_partial != Path::new(&cleaned_partial);
    let mut partial = expanded_partial.to_string_lossy().to_string();

    #[cfg(unix)]
    let path_separator = SEP;
    #[cfg(windows)]
    let path_separator = cleaned_partial
        .chars()
        .rfind(|c: &char| is_separator(*c))
        .unwrap_or(SEP);

    // Handle the trailing dot case
    if cleaned_partial.ends_with(&format!("{path_separator}.")) {
        partial.push_str(&format!("{path_separator}."));
    }

    let cwd_pathbufs: Vec<_> = cwds
        .iter()
        .map(|cwd| Path::new(cwd.as_ref()).to_path_buf())
        .collect();
    let ls_colors = (engine_state.config.completions.use_ls_colors
        && engine_state.config.use_ansi_coloring)
        .then(|| {
            let ls_colors_env_str = match stack.get_env_var(engine_state, "LS_COLORS") {
                Some(v) => env_to_string("LS_COLORS", v, engine_state, stack).ok(),
                None => None,
            };
            get_ls_colors(ls_colors_env_str)
        });

    let mut cwds = cwd_pathbufs.clone();
    let mut prefix_len = 0;
    let mut original_cwd = OriginalCwd::None;

    let mut components = Path::new(&partial).components().peekable();
    match components.peek().cloned() {
        Some(c @ Component::Prefix(..)) => {
            // windows only by definition
            cwds = vec![[c, Component::RootDir].iter().collect()];
            prefix_len = c.as_os_str().len();
            original_cwd = OriginalCwd::Prefix(c.as_os_str().to_string_lossy().into_owned());
        }
        Some(c @ Component::RootDir) => {
            // This is kind of a hack. When joining an empty string with the rest,
            // we add the slash automagically
            cwds = vec![PathBuf::from(c.as_os_str())];
            prefix_len = 1;
            original_cwd = OriginalCwd::Prefix(String::new());
        }
        Some(Component::Normal(home)) if home.to_string_lossy() == "~" => {
            cwds = home_dir()
                .map(|dir| vec![dir.into()])
                .unwrap_or(cwd_pathbufs);
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
        &cwds
            .into_iter()
            .map(|cwd| PathBuiltFromString {
                cwd,
                parts: Vec::new(),
                isdir: false,
            })
            .collect::<Vec<_>>(),
        options,
        want_directory,
        isdir,
    )
    .into_iter()
    .map(|mut p| {
        if should_collapse_dots {
            p = collapse_ndots(p);
        }
        let cwd = p.cwd.clone();
        let path = original_cwd.apply(p, path_separator);
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
        FileSuggestion {
            span,
            path: escape_path(path, want_directory),
            style,
            cwd,
        }
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
    let maybe_variable = path.starts_with('$');
    let maybe_number = path.parse::<f64>().is_ok();
    if filename_contaminated || dirname_contaminated || maybe_flag || maybe_variable || maybe_number
    {
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

/// Collapse multiple ".." components into n-dots.
///
/// It performs the reverse operation of `expand_ndots`, collapsing sequences of ".." into n-dots,
/// such as "..." and "....".
///
/// The resulting path will use platform-specific path separators, regardless of what path separators were used in the input.
fn collapse_ndots(path: PathBuiltFromString) -> PathBuiltFromString {
    let mut result = PathBuiltFromString {
        parts: Vec::with_capacity(path.parts.len()),
        isdir: path.isdir,
        cwd: path.cwd,
    };

    let mut dot_count = 0;

    for part in path.parts {
        if part == ".." {
            dot_count += 1;
        } else {
            if dot_count > 0 {
                result.parts.push(".".repeat(dot_count + 1));
                dot_count = 0;
            }
            result.parts.push(part);
        }
    }

    // Add any remaining dots
    if dot_count > 0 {
        result.parts.push(".".repeat(dot_count + 1));
    }

    result
}
