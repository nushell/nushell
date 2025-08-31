use std::collections::HashMap;

use crate::completions::{Completer, CompletionOptions, SemanticSuggestion, SuggestionKind};
use nu_protocol::{
    ENV_VARIABLE_ID, IN_VARIABLE_ID, NU_VARIABLE_ID, Span,
    engine::{Stack, StateWorkingSet},
};
use reedline::Suggestion;

use super::completion_options::NuMatcher;

pub struct VariableCompletion;

impl Completer for VariableCompletion {
    fn fetch(
        &mut self,
        working_set: &StateWorkingSet,
        _stack: &Stack,
        prefix: impl AsRef<str>,
        span: Span,
        offset: usize,
        options: &CompletionOptions,
    ) -> Vec<SemanticSuggestion> {
        let mut matcher = NuMatcher::new(prefix, options);
        let current_span = reedline::Span {
            start: span.start - offset,
            end: span.end - offset,
        };

        // Variable completion (e.g: $en<tab> to complete $env)
        let mut variables = HashMap::new();
        variables.insert("$nu".into(), &NU_VARIABLE_ID);
        variables.insert("$in".into(), &IN_VARIABLE_ID);
        variables.insert("$env".into(), &ENV_VARIABLE_ID);

        // TODO: The following can be refactored (see find_commands_by_predicate() used in
        // command_completions).
        let mut removed_overlays = vec![];
        // Working set scope vars
        for scope_frame in working_set.delta.scope.iter().rev() {
            for overlay_frame in scope_frame.active_overlays(&mut removed_overlays).rev() {
                for (name, var_id) in &overlay_frame.vars {
                    let name = String::from_utf8_lossy(name).to_string();
                    variables.insert(name, var_id);
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
                let name = String::from_utf8_lossy(name).to_string();
                variables.insert(name, var_id);
            }
        }

        for (name, var_id) in variables {
            matcher.add_semantic_suggestion(SemanticSuggestion {
                suggestion: Suggestion {
                    value: name,
                    span: current_span,
                    description: Some(working_set.get_variable(*var_id).ty.to_string()),
                    ..Suggestion::default()
                },
                kind: Some(SuggestionKind::Variable),
            });
        }

        matcher.results()
    }
}
