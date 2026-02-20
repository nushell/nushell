use std::collections::HashSet;

use crate::completions::{Completer, CompletionOptions};
use nu_protocol::{
    Category, Span, SuggestionKind,
    engine::{CommandType, Stack, StateWorkingSet},
};
use reedline::Suggestion;

use super::{SemanticSuggestion, completion_options::NuMatcher};

// Try to resolve a module by its head or by matching module names that end with
// `/head` (so `clip` will match `std/clip`). This is a local helper used only
// by the command completer to keep the lookup logic in one place and avoid
// duplicating traversal code.
fn find_module_by_head_or_suffix(
    working_set: &StateWorkingSet,
    head: &[u8],
) -> Option<nu_protocol::ModuleId> {
    // Exact lookup first
    if let Some(mid) = working_set.find_module(head) {
        return Some(mid);
    }

    let mut removed_overlays: Vec<Vec<u8>> = vec![];

    // Search delta scope overlays
    for scope_frame in working_set.delta.scope.iter().rev() {
        for overlay_frame in scope_frame.active_overlays(&mut removed_overlays).rev() {
            for (mname, &mid) in overlay_frame.modules.iter() {
                if mname.ends_with(head)
                    && (mname.len() == head.len() || mname[mname.len() - head.len() - 1] == b'/')
                {
                    return Some(mid);
                }
            }
        }
    }

    // Search permanent overlays
    for overlay_frame in working_set
        .permanent_state
        .active_overlays(&removed_overlays)
        .rev()
    {
        for (mname, &mid) in overlay_frame.modules.iter() {
            if mname.ends_with(head)
                && (mname.len() == head.len() || mname[mname.len() - head.len() - 1] == b'/')
            {
                return Some(mid);
            }
        }
    }

    None
}

// TODO: Add a toggle for quoting multi word commands. Useful for: `which` and `attr complete`
pub struct CommandCompletion {
    /// Whether to include internal commands
    pub internals: bool,
    /// Whether to include external commands
    pub externals: bool,
}

impl CommandCompletion {
    fn external_command_completion(
        &self,
        working_set: &StateWorkingSet,
        sugg_span: reedline::Span,
        internal_suggs: HashSet<String>,
        mut matcher: NuMatcher<SemanticSuggestion>,
    ) -> Vec<SemanticSuggestion> {
        let mut external_commands = HashSet::new();

        let paths_val = working_set.permanent_state.get_env_var("path");

        if let Some(paths_val) = paths_val
            && let Ok(paths) = paths_val.as_list()
        {
            for path in paths {
                let path = path.coerce_str().unwrap_or_default();

                if let Ok(mut contents) = std::fs::read_dir(path.as_ref()) {
                    while let Some(Ok(item)) = contents.next() {
                        if working_set
                            .permanent_state
                            .config
                            .completions
                            .external
                            .max_results
                            <= external_commands.len() as i64
                        {
                            break;
                        }
                        let Ok(name) = item.file_name().into_string() else {
                            continue;
                        };
                        // If there's an internal command with the same name, adds ^cmd to the
                        // matcher so that both the internal and external command are included
                        let value = if internal_suggs.contains(&name) {
                            format!("^{name}")
                        } else {
                            name.clone()
                        };
                        if external_commands.contains(&value) {
                            continue;
                        }
                        // TODO: check name matching before a relative heavy IO involved
                        // `is_executable` for performance consideration, should avoid
                        // duplicated `match_aux` call for matched items in the future
                        if matcher.check_match(&name).is_some()
                            && Self::is_executable_command(item.path())
                        {
                            external_commands.insert(value.clone());
                            matcher.add(
                                name,
                                SemanticSuggestion {
                                    suggestion: Suggestion {
                                        value,
                                        span: sugg_span,
                                        append_whitespace: true,
                                        ..Default::default()
                                    },
                                    kind: Some(SuggestionKind::Command(
                                        CommandType::External,
                                        None,
                                    )),
                                },
                            );
                        }
                    }
                }
            }
        }

        matcher.suggestion_results()
    }

    fn is_executable_command(path: impl AsRef<std::path::Path>) -> bool {
        let path = path.as_ref();
        if is_executable::is_executable(path) {
            return true;
        }

        if cfg!(windows)
            && let Some(ext) = path.extension()
        {
            return ext.eq_ignore_ascii_case("ps1") && path.is_file();
        }

        false
    }
}

impl Completer for CommandCompletion {
    fn fetch(
        &mut self,
        working_set: &StateWorkingSet,
        _stack: &Stack,
        prefix: impl AsRef<str>,
        span: Span,
        offset: usize,
        options: &CompletionOptions,
    ) -> Vec<SemanticSuggestion> {
        let mut res = Vec::new();

        let sugg_span = reedline::Span::new(span.start - offset, span.end - offset);

        let mut internal_suggs = HashSet::new();
        if self.internals {
            let mut matcher = NuMatcher::new(prefix.as_ref(), options, true);
            working_set.traverse_commands(|name, decl_id| {
                let name = String::from_utf8_lossy(name);
                let command = working_set.get_decl(decl_id);
                if command.signature().category == Category::Removed {
                    return;
                }
                let matched = matcher.add_semantic_suggestion(SemanticSuggestion {
                    suggestion: Suggestion {
                        value: name.to_string(),
                        description: Some(command.description().to_string()),
                        span: sugg_span,
                        append_whitespace: true,
                        ..Suggestion::default()
                    },
                    kind: Some(SuggestionKind::Command(
                        command.command_type(),
                        Some(decl_id),
                    )),
                });
                if matched {
                    internal_suggs.insert(name.to_string());
                }
            });
            // If the prefix is a module name (or a module name followed by a
            // trailing space), include module-qualified decls (e.g. `clip copy`)
            // even when the module overlay is active and top-level declarations
            // may have shadowed or changed visibility.
            let trimmed_prefix = prefix.as_ref().trim_end();
            if !trimmed_prefix.contains(' ') {
                let prefix_bytes = trimmed_prefix.as_bytes();

                if let Some(module_id) = find_module_by_head_or_suffix(working_set, prefix_bytes) {
                    let module = working_set.get_module(module_id);
                    for (full_name, decl_id) in module.decls_with_head(prefix_bytes) {
                        let name = String::from_utf8_lossy(&full_name).to_string();
                        if internal_suggs.contains(&name) {
                            continue;
                        }

                        // Skip removed/deprecated commands
                        let command = working_set.get_decl(decl_id);
                        if command.signature().category == Category::Removed {
                            continue;
                        }

                        let _ = matcher.add_semantic_suggestion(SemanticSuggestion {
                            suggestion: Suggestion {
                                value: name.clone(),
                                description: Some(command.description().to_string()),
                                span: sugg_span,
                                append_whitespace: true,
                                ..Suggestion::default()
                            },
                            kind: Some(SuggestionKind::Command(
                                command.command_type(),
                                Some(decl_id),
                            )),
                        });
                    }
                }
            }

            // Collect matcher results first
            res.extend(matcher.suggestion_results());

            // If the user typed a single token (e.g. `clip`) and we didn't produce any
            // module-qualified suggestions (e.g. `clip copy`), attempt a conservative
            // fallback by scanning permanent declarations for `prefix + ' '` entries.
            // This covers cases where overlays shadow the permanent decls.
            if !trimmed_prefix.contains(' ') {
                let prefix_space = format!("{} ", trimmed_prefix);
                let has_module_suggestion = res
                    .iter()
                    .any(|s| s.suggestion.value.starts_with(&prefix_space));

                if !has_module_suggestion {
                    working_set
                        .permanent_state
                        .traverse_commands(|name, decl_id| {
                            let name_str = String::from_utf8_lossy(name).to_string();
                            if name_str.starts_with(&prefix_space)
                                && !res.iter().any(|s| s.suggestion.value == name_str)
                            {
                                let command = working_set.permanent_state.get_decl(decl_id);
                                if command.signature().category == Category::Removed {
                                    return;
                                }

                                res.push(SemanticSuggestion {
                                    suggestion: Suggestion {
                                        value: name_str.clone(),
                                        description: Some(command.description().to_string()),
                                        span: sugg_span,
                                        append_whitespace: true,
                                        ..Suggestion::default()
                                    },
                                    kind: Some(SuggestionKind::Command(
                                        command.command_type(),
                                        Some(decl_id),
                                    )),
                                });
                            }
                        });
                }
            }
        }

        if self.externals {
            let external_suggs = self.external_command_completion(
                working_set,
                sugg_span,
                internal_suggs,
                NuMatcher::new(prefix, options, true),
            );
            res.extend(external_suggs);
        }

        res
    }
}
