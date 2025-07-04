use crate::completions::{
    Completer, CompletionOptions, SemanticSuggestion, SuggestionKind,
    completion_common::{FileSuggestion, surround_remove},
    completion_options::NuMatcher,
    file_path_completion,
};
use nu_path::expand_tilde;
use nu_protocol::{
    Span,
    engine::{Stack, StateWorkingSet, VirtualPath},
};
use reedline::Suggestion;
use std::{
    collections::HashSet,
    path::{MAIN_SEPARATOR_STR, PathBuf, is_separator},
};

pub struct DotNuCompletion {
    /// e.g. use std/a<tab>
    pub std_virtual_path: bool,
}

impl Completer for DotNuCompletion {
    fn fetch(
        &mut self,
        working_set: &StateWorkingSet,
        stack: &Stack,
        prefix: impl AsRef<str>,
        span: Span,
        offset: usize,
        options: &CompletionOptions,
    ) -> Vec<SemanticSuggestion> {
        let prefix_str = prefix.as_ref();
        let start_with_backquote = prefix_str.starts_with('`');
        let end_with_backquote = prefix_str.ends_with('`');
        let prefix_str = prefix_str.replace('`', "");
        // e.g. `./`, `..\`, `/`
        let not_lib_dirs = prefix_str
            .chars()
            .find(|c| *c != '.')
            .is_some_and(is_separator);
        let mut search_dirs: Vec<PathBuf> = vec![];

        let (base, partial) = if let Some((parent, remain)) = prefix_str.rsplit_once(is_separator) {
            // If prefix_str is only a word we want to search in the current dir.
            // "/xx" should be split to "/" and "xx".
            if parent.is_empty() {
                (MAIN_SEPARATOR_STR, remain)
            } else {
                (parent, remain)
            }
        } else {
            (".", prefix_str.as_str())
        };
        let base_dir = base.replace(is_separator, MAIN_SEPARATOR_STR);

        // Fetch the lib dirs
        // NOTE: 2 ways to setup `NU_LIB_DIRS`
        // 1. `const NU_LIB_DIRS = [paths]`, equal to `nu -I paths`
        // 2. `$env.NU_LIB_DIRS = [paths]`
        let const_lib_dirs = working_set
            .find_variable(b"$NU_LIB_DIRS")
            .and_then(|vid| working_set.get_variable(vid).const_val.as_ref());
        let env_lib_dirs = working_set.get_env_var("NU_LIB_DIRS");
        let lib_dirs: HashSet<PathBuf> = [const_lib_dirs, env_lib_dirs]
            .into_iter()
            .flatten()
            .flat_map(|lib_dirs| {
                lib_dirs
                    .as_list()
                    .into_iter()
                    .flat_map(|it| it.iter().filter_map(|x| x.to_path().ok()))
                    .map(expand_tilde)
            })
            .collect();

        // Check if the base_dir is a folder
        let cwd = working_set.permanent_state.cwd(None);
        if base_dir != "." {
            let expanded_base_dir = expand_tilde(&base_dir);
            let is_base_dir_relative = expanded_base_dir.is_relative();
            // Search in base_dir as well as lib_dirs.
            // After expanded, base_dir can be a relative path or absolute path.
            // If relative, we join "current working dir" with it to get subdirectory and add to search_dirs.
            // If absolute, we add it to search_dirs.
            if let Ok(mut cwd) = cwd {
                if is_base_dir_relative {
                    cwd.push(&base_dir);
                    search_dirs.push(cwd.into_std_path_buf());
                } else {
                    search_dirs.push(expanded_base_dir);
                }
            }
            if !not_lib_dirs {
                search_dirs.extend(lib_dirs.into_iter().map(|mut dir| {
                    dir.push(&base_dir);
                    dir
                }));
            }
        } else {
            if let Ok(cwd) = cwd {
                search_dirs.push(cwd.into_std_path_buf());
            }
            if !not_lib_dirs {
                search_dirs.extend(lib_dirs);
            }
        }

        // Fetch the files filtering the ones that ends with .nu
        // and transform them into suggestions
        let mut completions = file_path_completion(
            span,
            partial,
            &search_dirs
                .iter()
                .filter_map(|d| d.to_str())
                .collect::<Vec<_>>(),
            options,
            working_set.permanent_state,
            stack,
        );

        if self.std_virtual_path {
            let mut matcher = NuMatcher::new(partial, options);
            let base_dir = surround_remove(&base_dir);
            if base_dir == "." {
                let surround_prefix = partial
                    .chars()
                    .take_while(|c| "`'\"".contains(*c))
                    .collect::<String>();
                for path in ["std", "std-rfc"] {
                    let path = format!("{surround_prefix}{path}");
                    matcher.add(
                        path.clone(),
                        FileSuggestion {
                            span,
                            path,
                            style: None,
                            is_dir: true,
                        },
                    );
                }
            } else if let Some(VirtualPath::Dir(sub_paths)) =
                working_set.find_virtual_path(&base_dir)
            {
                for sub_vp_id in sub_paths {
                    let (path, sub_vp) = working_set.get_virtual_path(*sub_vp_id);
                    let path = path
                        .strip_prefix(&format!("{base_dir}/"))
                        .unwrap_or(path)
                        .to_string();
                    matcher.add(
                        path.clone(),
                        FileSuggestion {
                            path,
                            span,
                            style: None,
                            is_dir: matches!(sub_vp, VirtualPath::Dir(_)),
                        },
                    );
                }
            }
            completions.extend(matcher.results());
        }

        completions
            .into_iter()
            // Different base dir, so we list the .nu files or folders
            .filter(|it| {
                // for paths with spaces in them
                let path = it.path.trim_end_matches('`');
                path.ends_with(".nu") || it.is_dir
            })
            .map(|x| {
                let append_whitespace = !x.is_dir && (!start_with_backquote || end_with_backquote);
                // Re-calculate the span to replace
                let mut span_offset = 0;
                let mut value = x.path.to_string();
                // Complete only the last path component
                if base_dir == MAIN_SEPARATOR_STR {
                    span_offset = base_dir.len()
                } else if base_dir != "." {
                    span_offset = base_dir.len() + 1
                }
                // Retain only one '`'
                if start_with_backquote {
                    value = value.trim_start_matches('`').to_string();
                    span_offset += 1;
                }
                // Add the backquote back
                if end_with_backquote && !value.ends_with('`') {
                    value.push('`');
                }
                let end = x.span.end - offset;
                let start = std::cmp::min(end, x.span.start - offset + span_offset);
                SemanticSuggestion {
                    suggestion: Suggestion {
                        value,
                        style: x.style,
                        span: reedline::Span { start, end },
                        append_whitespace,
                        ..Suggestion::default()
                    },
                    kind: Some(SuggestionKind::Module),
                }
            })
            .collect::<Vec<_>>()
    }
}
