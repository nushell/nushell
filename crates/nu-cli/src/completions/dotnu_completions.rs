use crate::completions::{
    Completer, CompletionOptions, SemanticSuggestion, SuggestionKind,
    completion_common::FileSuggestion, completion_options::NuMatcher,
};
use nu_path::expand_tilde;
use nu_protocol::{
    Span,
    engine::{Stack, StateWorkingSet, VirtualPath},
};
use reedline::Suggestion;
use std::collections::HashSet;

use super::completion_common::{complete_item, surround_remove};

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
        // Fetch the lib dirs
        // NOTE: 2 ways to setup `NU_LIB_DIRS`
        // 1. `const NU_LIB_DIRS = [paths]`, equal to `nu -I paths`
        // 2. `$env.NU_LIB_DIRS = [paths]`
        let const_lib_dirs = working_set
            .find_variable(b"$NU_LIB_DIRS")
            .and_then(|vid| working_set.get_variable(vid).const_val.as_ref());
        let env_lib_dirs = working_set.get_env_var("NU_LIB_DIRS");
        let mut search_dirs = [const_lib_dirs, env_lib_dirs]
            .into_iter()
            .flatten()
            .flat_map(|lib_dirs| {
                lib_dirs
                    .as_list()
                    .into_iter()
                    .flat_map(|it| it.iter().filter_map(|x| x.to_path().ok()))
                    .map(expand_tilde)
            })
            .collect::<HashSet<_>>();

        if let Ok(cwd) = working_set.permanent_state.cwd(None) {
            search_dirs.insert(cwd.into_std_path_buf());
        }

        let mut completions = Vec::new();

        // Add std virtual paths first
        if self.std_virtual_path {
            let surround_prefix = prefix
                .as_ref()
                .chars()
                .take_while(|c| "`'\"".contains(*c))
                .collect::<String>();
            let mut matcher = NuMatcher::new(&prefix, options);
            // Where we have '/' in the prefix, e.g. use std/l
            if let Some((base_dir, _)) = prefix.as_ref().rsplit_once("/") {
                let base_dir = surround_remove(base_dir);
                if let Some(VirtualPath::Dir(sub_paths)) = working_set.find_virtual_path(&base_dir)
                {
                    for sub_vp_id in sub_paths {
                        let (path, sub_vp) = working_set.get_virtual_path(*sub_vp_id);
                        let path = format!("{surround_prefix}{path}");
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
            } else {
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
            }
            completions.extend(matcher.results());
        }

        // Fetch the files
        completions.extend(complete_item(
            false,
            span,
            prefix.as_ref(),
            &search_dirs
                .iter()
                .filter_map(|d| d.to_str())
                .collect::<Vec<_>>(),
            options,
            working_set.permanent_state,
            stack,
        ));

        let into_suggestion = |x: &FileSuggestion| SemanticSuggestion {
            suggestion: Suggestion {
                value: x.path.to_string(),
                style: x.style,
                span: reedline::Span {
                    start: x.span.start - offset,
                    end: x.span.end - offset,
                },
                append_whitespace: !x.is_dir,
                ..Suggestion::default()
            },
            kind: Some(SuggestionKind::Module),
        };

        // Put files atop
        completions
            .iter()
            // filtering the files that ends with .nu
            .filter(|it| {
                // for paths with spaces in them
                let path = it.path.trim_end_matches('`');
                path.ends_with(".nu")
            })
            // or directories
            .chain(completions.iter().filter(|it| it.is_dir))
            .map(into_suggestion)
            .collect::<Vec<_>>()
    }
}
