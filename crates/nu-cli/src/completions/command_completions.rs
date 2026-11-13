use std::collections::HashSet;

use crate::completions::{Completer, Context, Fetched, to_reedline_span};
use nu_protocol::{
    Category, DeclId, SuggestionKind,
    engine::{CommandType, StateWorkingSet},
};
use reedline::Suggestion;

use super::{SemanticSuggestion, completion_options::NuMatcher};

/// Which command declarations a [`CommandCompletion`] offers.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CommandScope {
    /// Internal commands (all visible) plus external `PATH` commands — the default head scope.
    All,
    /// External `PATH` commands only (the `^` sigil).
    ExternalsOnly,
    /// Built-in commands only, scanning even shadowed declarations (the `%` sigil).
    BuiltinsOnly,
    /// Internal commands only — never scans `PATH` (subcommands, `attr complete`).
    InternalsOnly,
}

impl CommandScope {
    fn externals(self) -> bool {
        matches!(self, CommandScope::All | CommandScope::ExternalsOnly)
    }

    /// Narrowed by `$env.config.completions.external.enable`: only [`All`](Self::All)
    /// collapses to [`InternalsOnly`](Self::InternalsOnly) when externals are disabled.
    fn enabled_in(self, context: &Context) -> Self {
        let enabled = context
            .working_set
            .permanent_state
            .config
            .completions
            .external
            .enable;
        match (self, enabled) {
            (CommandScope::All, false) => CommandScope::InternalsOnly,
            (scope, _) => scope,
        }
    }
}

pub struct CommandCompletion {
    /// Which declarations to offer.
    scope: CommandScope,
    /// Whether to quote space-separated internal command names.
    quote_internals: bool,
}

impl CommandCompletion {
    /// Offer `scope`, leaving internal command names unquoted.
    pub(crate) fn new(scope: CommandScope) -> Self {
        Self {
            scope,
            quote_internals: false,
        }
    }

    /// Offer `scope`, quoting space-separated internal command names.
    pub(crate) fn quoted(scope: CommandScope) -> Self {
        Self {
            scope,
            quote_internals: true,
        }
    }

    /// Lazily yields `(file name, path)` for each entry across `PATH`.
    fn get_executable_files<'a>(
        &self,
        working_set: &'a StateWorkingSet,
    ) -> impl Iterator<Item = (String, std::path::PathBuf)> + 'a {
        working_set
            .permanent_state
            .get_env_var("path")
            .and_then(|path_value| path_value.as_list().ok())
            .into_iter()
            .flatten()
            .map(|path_value| path_value.coerce_str().unwrap_or_default())
            .filter_map(|directory_path| std::fs::read_dir(directory_path.as_ref()).ok())
            .flatten()
            .filter_map(Result::ok)
            .filter_map(|directory_entry| {
                Some((
                    directory_entry.file_name().into_string().ok()?,
                    directory_entry.path(),
                ))
            })
    }

    fn is_executable_command(path: impl AsRef<std::path::Path>) -> bool {
        let path = path.as_ref();

        is_executable::is_executable(path)
            || (cfg!(windows)
                && path.is_file()
                && path
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("ps1")))
    }

    /// Collects built-in commands, including shadowed ones (reverse traversal).
    fn collect_builtins(
        &self,
        context: &Context,
        suggestion_span: reedline::Span,
    ) -> (Vec<SemanticSuggestion>, HashSet<String>) {
        let working_set = context.working_set;
        let mut matcher = NuMatcher::new(context.prefix_str(), context.options, true);
        let mut internal_names = HashSet::new();
        let mut seen_names: HashSet<&str> = HashSet::new();

        (0..working_set.num_decls())
            .rev()
            .map(DeclId::new)
            .map(|declaration_id| (declaration_id, working_set.get_decl(declaration_id)))
            .filter(|(_, command)| {
                command.signature().category != Category::Removed
                    && command.command_type() == CommandType::Builtin
                    && seen_names.insert(command.name())
            })
            .for_each(|(declaration_id, command)| {
                // As in `collect_visible_internals`: match before allocating a description.
                if matcher.check_match(command.name()).is_none() {
                    return;
                }
                let name = command.name().to_string();
                let suggestion = SemanticSuggestion {
                    suggestion: Suggestion {
                        value: name.clone(),
                        description: Some(command.description().to_string()),
                        span: suggestion_span,
                        append_whitespace: true,
                        ..Suggestion::default()
                    },
                    kind: Some(SuggestionKind::Command(
                        CommandType::Builtin,
                        Some(declaration_id),
                    )),
                };

                if matcher.add_semantic_suggestion(suggestion) {
                    internal_names.insert(name);
                }
            });

        (matcher.suggestion_results(), internal_names)
    }

    /// Scans internal commands using the engine's built-in traversal
    fn collect_visible_internals(
        &self,
        context: &Context,
        suggestion_span: reedline::Span,
    ) -> (Vec<SemanticSuggestion>, HashSet<String>) {
        let working_set = context.working_set;
        let mut matcher = NuMatcher::new(context.prefix_str(), context.options, true);
        let mut internal_names = HashSet::new();

        working_set.traverse_commands(|name_bytes, declaration_id| {
            let command = working_set.get_decl(declaration_id);
            if command.signature().category == Category::Removed {
                return;
            }

            let raw_name = String::from_utf8_lossy(name_bytes);
            let name = match self.quote_internals && nu_utils::needs_quoting(&raw_name) {
                true => nu_utils::escape_quote_string(&raw_name),
                false => raw_name.into_owned(),
            };

            // Match the prefix before `description()` allocates (runs every keystroke).
            if matcher.check_match(&name).is_none() {
                return;
            }

            let suggestion = SemanticSuggestion {
                suggestion: Suggestion {
                    value: name.clone(),
                    description: Some(command.description().to_string()),
                    span: suggestion_span,
                    append_whitespace: true,
                    ..Suggestion::default()
                },
                kind: Some(SuggestionKind::Command(
                    command.command_type(),
                    Some(declaration_id),
                )),
            };

            if matcher.add_semantic_suggestion(suggestion) {
                internal_names.insert(name);
            }
        });

        (matcher.suggestion_results(), internal_names)
    }

    /// Walks `PATH` once, offering collisions `^`-prefixed and collecting external matches.
    fn process_external_commands(
        &self,
        context: &Context,
        suggestion_span: reedline::Span,
        internal_suggestions: &mut Vec<SemanticSuggestion>,
        internal_names: &HashSet<String>,
    ) -> Vec<SemanticSuggestion> {
        let working_set = context.working_set;
        let maximum_results = working_set
            .permanent_state
            .config
            .completions
            .external
            .max_results as usize;
        let mut matcher = NuMatcher::new(context.prefix_str(), context.options, true);

        let mut external_commands: HashSet<String> = HashSet::new();
        let mut collisions: HashSet<String> = HashSet::new();

        for (file_name, file_path) in self.get_executable_files(working_set) {
            let is_collision =
                internal_names.contains(&file_name) && !collisions.contains(&file_name);
            let wants_suggestion = external_commands.len() < maximum_results
                && matcher.check_match(&file_name).is_some();

            // `is_executable_command` stats the file, which dominates the scan on slow
            // filesystems (e.g. WSL's 9P mounts to Windows `PATH` directories), so only
            // pay for entries that can still contribute a suggestion or a collision.
            if !(wants_suggestion || is_collision) || !Self::is_executable_command(&file_path) {
                continue;
            }

            if is_collision {
                collisions.insert(file_name.clone());
            }

            if wants_suggestion {
                let command_value = match internal_names.contains(&file_name) {
                    true => format!("^{file_name}"),
                    false => file_name.clone(),
                };

                if external_commands.insert(command_value.clone()) {
                    matcher.add(
                        file_name,
                        SemanticSuggestion {
                            suggestion: Suggestion {
                                value: command_value,
                                span: suggestion_span,
                                append_whitespace: true,
                                ..Suggestion::default()
                            },
                            kind: Some(SuggestionKind::Command(CommandType::External, None)),
                        },
                    );
                }
            }
        }

        // Add `%`-prefixed copies of collided internal suggestions.
        let percent_prefixed_suggestions: Vec<SemanticSuggestion> = internal_suggestions
            .iter()
            .filter(|suggestion| collisions.contains(&suggestion.suggestion.value))
            .map(|suggestion| {
                let mut prefixed = suggestion.suggestion.clone();
                prefixed.value = format!("%{}", suggestion.suggestion.value);
                SemanticSuggestion {
                    suggestion: prefixed,
                    kind: suggestion.kind.clone(),
                }
            })
            .collect();

        internal_suggestions.extend(percent_prefixed_suggestions);
        matcher.suggestion_results()
    }
}

impl Completer for CommandCompletion {
    fn fetch(&mut self, context: &Context) -> Fetched {
        let suggestion_span = to_reedline_span(context.span, context.offset);
        let scope = self.scope.enabled_in(context);

        let (mut suggestions, internal_names) = match scope {
            CommandScope::ExternalsOnly => (Vec::new(), HashSet::new()),
            CommandScope::BuiltinsOnly => self.collect_builtins(context, suggestion_span),
            CommandScope::All | CommandScope::InternalsOnly => {
                self.collect_visible_internals(context, suggestion_span)
            }
        };

        // Only a `PATH` scan is expensive enough to be worth caching.
        let externals = scope.externals();
        if externals {
            let external_suggestions = self.process_external_commands(
                context,
                suggestion_span,
                &mut suggestions,
                &internal_names,
            );
            suggestions.extend(external_suggestions);
        }

        match externals {
            true => Fetched::answering(suggestions).worth_keeping(),
            false => Fetched::answering(suggestions),
        }
    }
}
