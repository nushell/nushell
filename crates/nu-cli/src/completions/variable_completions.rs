use crate::completions::{Completer, CompletionOptions, SemanticSuggestion, SuggestionKind};
use nu_protocol::{
    engine::{Stack, StateWorkingSet},
    Span, VarId,
};
use reedline::Suggestion;

use super::completion_options::NuMatcher;

pub struct VariableCompletion {}

impl Completer for VariableCompletion {
    fn fetch(
        &mut self,
        working_set: &StateWorkingSet,
        _stack: &Stack,
        prefix: &[u8],
        span: Span,
        offset: usize,
        _pos: usize,
        options: &CompletionOptions,
    ) -> Vec<SemanticSuggestion> {
        let prefix_str = String::from_utf8_lossy(prefix);
        let mut matcher = NuMatcher::new(prefix_str, options.clone());
        let current_span = reedline::Span {
            start: span.start - offset,
            end: span.end - offset,
        };

        // Variable completion (e.g: $en<tab> to complete $env)
        let builtins = ["$nu", "$in", "$env"];
        for builtin in builtins {
            matcher.add_semantic_suggestion(SemanticSuggestion {
                suggestion: Suggestion {
                    value: builtin.to_string(),
                    span: current_span,
                    ..Suggestion::default()
                },
                // TODO is there a way to get the VarId to get the type???
                kind: None,
            });
        }

        let mut add_candidate = |name, var_id: &VarId| {
            matcher.add_semantic_suggestion(SemanticSuggestion {
                suggestion: Suggestion {
                    value: String::from_utf8_lossy(name).to_string(),
                    span: current_span,
                    ..Suggestion::default()
                },
                kind: Some(SuggestionKind::Type(
                    working_set.get_variable(*var_id).ty.clone(),
                )),
            })
        };

        // TODO: The following can be refactored (see find_commands_by_predicate() used in
        // command_completions).
        let mut removed_overlays = vec![];
        // Working set scope vars
        for scope_frame in working_set.delta.scope.iter().rev() {
            for overlay_frame in scope_frame.active_overlays(&mut removed_overlays).rev() {
                for (name, var_id) in &overlay_frame.vars {
                    add_candidate(name, var_id);
                }
            }
        }
        // Permanent state vars
        // for scope in &self.engine_state.scope {
        for overlay_frame in working_set
            .permanent_state
            .active_overlays(&removed_overlays)
            .rev()
        {
            for (name, var_id) in &overlay_frame.vars {
                add_candidate(name, var_id);
            }
        }

        matcher.results()
    }
}
