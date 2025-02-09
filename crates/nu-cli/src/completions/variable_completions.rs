use crate::completions::{Completer, CompletionOptions, SemanticSuggestion, SuggestionKind};
use nu_engine::{column::get_columns, eval_variable};
use nu_protocol::{
    ast::{Expr, FullCellPath, PathMember},
    engine::{Stack, StateWorkingSet},
    eval_const::eval_constant,
    Span, Value, VarId,
};
use reedline::Suggestion;

use super::completion_options::NuMatcher;

pub struct VariableNameCompletion {}

impl Completer for VariableNameCompletion {
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

pub struct CellPathCompletion<'a> {
    pub full_cell_path: &'a FullCellPath,
}

impl Completer for CellPathCompletion<'_> {
    fn fetch(
        &mut self,
        working_set: &StateWorkingSet,
        stack: &Stack,
        _prefix: &[u8],
        _span: Span,
        offset: usize,
        _pos: usize,
        options: &CompletionOptions,
    ) -> Vec<SemanticSuggestion> {
        // empty tail is already handled as variable names completion
        let Some((prefix_member, path_members)) = self.full_cell_path.tail.split_last() else {
            return vec![];
        };
        let (mut prefix_str, span) = match prefix_member {
            PathMember::String { val, span, .. } => (val.clone(), span),
            PathMember::Int { val, span, .. } => (val.to_string(), span),
        };
        // strip the placeholder
        prefix_str.pop();
        let true_end = std::cmp::max(span.start, span.end - 1);
        let span = Span::new(span.start, true_end);
        let current_span = reedline::Span {
            start: span.start - offset,
            end: true_end - offset,
        };

        let mut matcher = NuMatcher::new(prefix_str, options.clone());

        // evaluate the head expression to get its value
        let value = if let Expr::Var(var_id) = self.full_cell_path.head.expr {
            working_set
                .get_variable(var_id)
                .const_val
                .to_owned()
                .or_else(|| eval_variable(working_set.permanent_state, stack, var_id, span).ok())
        } else {
            eval_constant(working_set, &self.full_cell_path.head).ok()
        }
        .unwrap_or_default();

        for suggestion in nested_suggestions(&value, path_members, current_span) {
            matcher.add_semantic_suggestion(suggestion);
        }
        matcher.results()
    }
}

// Find recursively the values for cell_path
fn nested_suggestions(
    val: &Value,
    path_members: &[PathMember],
    current_span: reedline::Span,
) -> Vec<SemanticSuggestion> {
    let value = val
        .clone()
        .follow_cell_path(path_members, false)
        .unwrap_or_default();

    let kind = SuggestionKind::Type(value.get_type());
    let str_to_suggestion = |s: String| SemanticSuggestion {
        suggestion: Suggestion {
            value: s,
            span: current_span,
            ..Suggestion::default()
        },
        kind: Some(kind.to_owned()),
    };
    match value {
        Value::Record { val, .. } => val
            .columns()
            .map(|s| str_to_suggestion(s.to_string()))
            .collect(),
        Value::List { vals, .. } => get_columns(vals.as_slice())
            .into_iter()
            .map(str_to_suggestion)
            .collect(),
        _ => vec![],
    }
}
