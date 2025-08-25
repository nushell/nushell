use std::collections::HashMap;

use crate::completions::{Completer, CompletionOptions, SemanticSuggestion, SuggestionKind};
use nu_protocol::{
    Span, Type, VarId,
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
        let builtins = ["$nu", "$in", "$env"];
        for builtin in builtins {
            matcher.add_semantic_suggestion(SemanticSuggestion {
                suggestion: Suggestion {
                    value: builtin.to_string(),
                    span: current_span,
                    description: Some("reserved".into()),
                    ..Suggestion::default()
                },
                kind: Some(SuggestionKind::Variable),
            });
        }

        let mut add_candidate = |name, ty: &Type| {
            let mut value = String::from_utf8_lossy(name).to_string();
            if !value.starts_with('$') {
                value.insert(0, '$');
            }
            matcher.add_semantic_suggestion(SemanticSuggestion {
                suggestion: Suggestion {
                    value,
                    span: current_span,
                    description: Some(ty.to_string()),
                    ..Suggestion::default()
                },
                kind: Some(SuggestionKind::Variable),
            })
        };

        // TODO: smarter scope-aware variable completion
        let mut variables_defined_before_cursor = HashMap::new();
        // Permanent state vars
        for overlay_frame in working_set.permanent_state.active_overlays(&[]).rev() {
            for (name, var_id) in &overlay_frame.vars {
                let var = working_set.get_variable(*var_id);
                add_candidate(name, &var.ty);
            }
        }

        // A superset of valid variables in current scope,
        // A workaround for https://github.com/nushell/nushell/issues/15291
        let working_span = working_set
            .files()
            .last()
            .map(|f| f.covered_span)
            .unwrap_or(Span::unknown());
        for id in 0..working_set.num_vars() {
            let var_id = VarId::new(id);
            let var = working_set.get_variable(var_id);
            if working_span.contains_span(var.declaration_span)
                && var.declaration_span.start < span.end
            {
                let var_name = working_set.get_span_contents(var.declaration_span);
                variables_defined_before_cursor.insert(var_name, &var.ty);
            }
        }

        for (name, ty) in variables_defined_before_cursor {
            add_candidate(name, ty);
        }

        matcher.results()
    }
}
