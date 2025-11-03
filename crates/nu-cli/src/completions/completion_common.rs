use super::{MatchAlgorithm, completion_options::NuMatcher};
use crate::completions::CompletionOptions;
use nu_ansi_term::Style;
use nu_engine::env_to_string;
use nu_path::dots::expand_ndots;
use nu_path::{expand_to_real_path, home_dir};
use nu_protocol::{
    Span,
    engine::{EngineState, Stack, StateWorkingSet},
};
use nu_utils::IgnoreCaseExt;
use nu_utils::get_ls_colors;
use std::path::{Component, MAIN_SEPARATOR as SEP, Path, PathBuf, is_separator};

#[derive(Clone, Default)]
pub struct PathBuiltFromString {
    cwd: PathBuf,
    parts: Vec<String>,
    isdir: bool,
}

/// Recursively goes through paths that match a given `partial`.
/// built: State struct for a valid matching path built so far.
///
/// `want_directory`: Whether we want only directories as completion matches.
/// Some commands like `cd` can only be run on directories whereas others
/// like `ls` can be run on regular files as well.
///
/// `isdir`: whether the current partial path has a trailing slash.
/// Parsing a path string into a pathbuf loses that bit of information.
///
/// `enable_exact_match`: Whether match algorithm is Prefix and all previous components
/// of the path matched a directory exactly.
fn complete_rec(
    partial: &[&str],
    built_paths: &[PathBuiltFromString],
    options: &CompletionOptions,
    want_directory: bool,
    isdir: bool,
    enable_exact_match: bool,
) -> Vec<PathBuiltFromString> {
    let has_more = !partial.is_empty() && (partial.len() > 1 || isdir);

    if let Some((&base, rest)) = partial.split_first()
        && base.chars().all(|c| c == '.')
        && has_more
    {
        let built_paths: Vec<_> = built_paths
            .iter()
            .map(|built| {
                let mut built = built.clone();
                built.parts.push(base.to_string());
                built.isdir = true;
                built
            })
            .collect();
        return complete_rec(
            rest,
            &built_paths,
            options,
            want_directory,
            isdir,
            enable_exact_match,
        );
    }

    let prefix = partial.first().unwrap_or(&"");
    let mut matcher = NuMatcher::new(prefix, options);

    let mut exact_match = None;
    // Only relevant for case insensitive matching
    let mut multiple_exact_matches = false;
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
            // Symlinks to directories shouldn't have a trailing slash (#13275)
            built.isdir = entry_isdir && !entry.path().is_symlink();

            if !want_directory || entry_isdir {
                if enable_exact_match && !multiple_exact_matches && has_more {
                    let matches = if options.case_sensitive {
                        entry_name.eq(prefix)
                    } else {
                        entry_name.eq_ignore_case(prefix)
                    };
                    if matches {
                        if exact_match.is_none() {
                            exact_match = Some(built.clone());
                        } else {
                            multiple_exact_matches = true;
                        }
                    }
                }

                matcher.add(entry_name, built);
            }
        }
    }

    // Don't show longer completions if we have a single exact match (#13204, #14794)
    if !multiple_exact_matches && let Some(built) = exact_match {
        return complete_rec(
            &partial[1..],
            &[built],
            options,
            want_directory,
            isdir,
            true,
        );
    }

    if has_more {
        let mut completions = vec![];
        for built in matcher.results() {
            completions.extend(complete_rec(
                &partial[1..],
                &[built],
                options,
                want_directory,
                isdir,
                false,
            ));
        }
        completions
    } else {
        matcher.results()
    }
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

pub fn surround_remove(partial: &str) -> String {
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
    pub is_dir: bool,
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
        && engine_state.config.use_ansi_coloring.get(engine_state))
    .then(|| {
        let ls_colors_env_str = stack
            .get_env_var(engine_state, "LS_COLORS")
            .and_then(|v| env_to_string("LS_COLORS", v, engine_state, stack).ok());
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
        options.match_algorithm == MatchAlgorithm::Prefix,
    )
    .into_iter()
    .map(|mut p| {
        if should_collapse_dots {
            p = collapse_ndots(p);
        }
        let is_dir = p.isdir;
        let path = original_cwd.apply(p, path_separator);
        let real_path = expand_to_real_path(&path);
        let metadata = std::fs::symlink_metadata(&real_path).ok();
        let style = ls_colors.as_ref().map(|lsc| {
            lsc.style_for_path_with_metadata(&real_path, metadata.as_ref())
                .map(lscolors::Style::to_nu_ansi_term_style)
                .unwrap_or_default()
        });
        FileSuggestion {
            span,
            path: escape_path(path),
            style,
            is_dir,
        }
    })
    .collect()
}

// Fix files or folders with quotes or hashes
pub fn escape_path(path: String) -> String {
    // make glob pattern have the highest priority.
    if nu_glob::is_glob(path.as_str()) || path.contains('`') {
        // expand home `~` for https://github.com/nushell/nushell/issues/13905
        let pathbuf = nu_path::expand_tilde(path);
        let path = pathbuf.to_string_lossy();
        if path.contains('\'') {
            // decide to use double quotes
            // Path as Debug will do the escaping for `"`, `\`
            format!("{path:?}")
        } else {
            format!("'{path}'")
        }
    } else {
        let contaminated =
            path.contains(['\'', '"', ' ', '#', '(', ')', '{', '}', '[', ']', '|', ';']);
        let maybe_flag = path.starts_with('-');
        let maybe_variable = path.starts_with('$');
        let maybe_number = path.parse::<f64>().is_ok();
        if contaminated || maybe_flag || maybe_variable || maybe_number {
            format!("`{path}`")
        } else {
            path
        }
    }
}

pub struct AdjustView {
    pub prefix: String,
    pub span: Span,
    pub readjusted: bool,
}

pub fn adjust_if_intermediate(
    prefix: &str,
    working_set: &StateWorkingSet,
    mut span: nu_protocol::Span,
) -> AdjustView {
    let span_contents = String::from_utf8_lossy(working_set.get_span_contents(span)).to_string();
    let mut prefix = prefix.to_string();

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
