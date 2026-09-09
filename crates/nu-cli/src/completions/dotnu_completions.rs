use crate::completions::{
    Completer, CompletionOptions, Context, Fetched, MatchAlgorithm, SemanticSuggestion,
    completion_common::FileSuggestion, completion_options::NuMatcher, to_reedline_span,
};
use nu_path::expand_tilde;
use nu_protocol::{SuggestionKind, engine::VirtualPath};
use reedline::Suggestion;
use std::collections::{HashMap, HashSet};

use super::completion_common::{complete_item, surround_remove};

pub struct DotNuCompletion {
    /// e.g. use std/a<tab>
    pub std_virtual_path: bool,
}

impl DotNuCompletion {
    fn complete_with_options(
        &self,
        ctx: &Context,
        options: &CompletionOptions,
    ) -> Vec<SemanticSuggestion> {
        let working_set = ctx.working_set;
        let stack = ctx.stack;
        let span = ctx.span;
        let prefix = ctx.prefix_str();
        let reedline_span = to_reedline_span(span, ctx.offset);
        // Modules that are already loaded go first
        let mut matcher = NuMatcher::new(&prefix, options, true);
        let mut modules_map = HashMap::new();
        // TODO: inline-defined modules, e.g. `module foo {}; use foo<tab>` ?
        for overlay_frame in working_set.permanent_state.active_overlays(&[]) {
            modules_map.extend(&overlay_frame.modules);
        }

        for (module_name_bytes, module_id) in modules_map.into_iter() {
            let value = String::from_utf8_lossy(module_name_bytes).into_owned();
            // TODO: this is a quickfix. just like `help modules`, this constructs the module
            // description from scratch each time. we should construct module descriptions when the
            // module is first parsed and store it for later use (like we do with custom commands)
            // TODO: we need to provide descriptions not just for modules that are already in scope,
            // but for modules that haven't been imported yet.
            // (importing a new file like `use foo.nu`)
            let description = working_set
                .get_module_comments(*module_id)
                .map(|spans| working_set.build_desc(spans).0);

            matcher.add_semantic_suggestion(SemanticSuggestion {
                suggestion: Suggestion {
                    value,
                    description,
                    span: reedline_span,
                    append_whitespace: true,
                    ..Suggestion::default()
                },
                kind: Some(SuggestionKind::Module),
            });
        }

        // Add std virtual paths first
        let mut resolved_virtual_dir = false;
        if self.std_virtual_path {
            // Where we have '/' in the prefix, e.g. use std/l
            if let Some((base_dir, _)) = prefix.as_ref().rsplit_once("/") {
                let base_dir = surround_remove(base_dir);
                if let Some(VirtualPath::Dir(sub_paths)) = working_set.find_virtual_path(&base_dir)
                {
                    resolved_virtual_dir = true;
                    for sub_vp_id in sub_paths {
                        let (path, sub_vp) = working_set.get_virtual_path(*sub_vp_id);
                        matcher.add_semantic_suggestion(SemanticSuggestion {
                            suggestion: Suggestion {
                                value: path.into(),
                                span: reedline_span,
                                append_whitespace: !matches!(sub_vp, VirtualPath::Dir(_)),
                                ..Suggestion::default()
                            },
                            kind: Some(SuggestionKind::Module),
                        });
                    }
                }
            } else {
                for path in ["std", "std-rfc"] {
                    matcher.add_semantic_suggestion(SemanticSuggestion {
                        suggestion: Suggestion {
                            value: path.into(),
                            span: reedline_span,
                            ..Suggestion::default()
                        },
                        kind: Some(SuggestionKind::Module),
                    });
                }
            }
        }

        let mut all_results = matcher.suggestion_results();

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

        // A virtual dir (`use std/`) already supplied its children. Walking cwd
        // is the `/` hitch in large trees and cannot complete extra files under
        // virtual `std` (find_in_dirs prefers the VFS). Still search NU_LIB_DIRS
        // so a real `std/` overlay in lib dirs can appear.
        if !resolved_virtual_dir && let Ok(cwd) = working_set.permanent_state.cwd(None) {
            search_dirs.insert(cwd.into_std_path_buf());
        }

        // Fetch the files
        let module_file_results = complete_item(
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
        );

        all_results.extend(
            // Put files atop
            module_file_results
                .iter()
                // filtering the files that ends with .nu
                .filter(|it| {
                    // for paths with spaces in them
                    let path = it.path.trim_end_matches('`');
                    path.ends_with(".nu")
                })
                // or directories
                .chain(module_file_results.iter().filter(|it| it.is_dir))
                .map(|x: &FileSuggestion| SemanticSuggestion {
                    suggestion: Suggestion {
                        value: x.path.to_string(),
                        style: x.style,
                        span: reedline_span,
                        append_whitespace: !x.is_dir,
                        display_override: x.display_override.clone(),
                        match_indices: Some(x.match_indices.clone()),
                        ..Suggestion::default()
                    },
                    kind: Some(SuggestionKind::Module),
                })
                .collect::<Vec<_>>(),
        );

        all_results
    }
}

impl Completer for DotNuCompletion {
    fn fetch(&mut self, ctx: &Context) -> Fetched {
        let suggestions = if ctx.options.match_algorithm == MatchAlgorithm::Fallback {
            let mut options = ctx.options.clone();
            options.match_algorithm = MatchAlgorithm::Prefix;

            let prefix_matches = self.complete_with_options(ctx, &options);
            if prefix_matches.is_empty() {
                options.match_algorithm = MatchAlgorithm::Fuzzy;
                self.complete_with_options(ctx, &options)
            } else {
                prefix_matches
            }
        } else {
            self.complete_with_options(ctx, ctx.options)
        };

        Fetched::Cacheable(suggestions)
    }
}
