use crate::completions::{Completer, CompletionOptions, SemanticSuggestion, SuggestionKind};
use nu_engine::{column::get_columns, eval_variable};
use nu_protocol::{
    ast::{Expr, Expression, FullCellPath, PathMember},
    engine::{Stack, StateWorkingSet},
    eval_const::eval_constant,
    ShellError, Span, Value,
};
use reedline::Suggestion;

use super::completion_options::NuMatcher;

pub struct CellPathCompletion<'a> {
    pub full_cell_path: &'a FullCellPath,
}

impl Completer for CellPathCompletion<'_> {
    fn fetch(
        &mut self,
        working_set: &StateWorkingSet,
        stack: &Stack,
        _prefix: impl AsRef<str>,
        _span: Span,
        offset: usize,
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

        let mut matcher = NuMatcher::new(prefix_str, options);
        let value = eval_cell_path(
            working_set,
            stack,
            &self.full_cell_path.head,
            path_members,
            span,
        )
        .unwrap_or_default();

        for suggestion in get_suggestions_by_value(&value, current_span) {
            matcher.add_semantic_suggestion(suggestion);
        }
        matcher.results()
    }
}

/// Follow cell path to get the value
/// NOTE: This is a relatively lightweight implementation,
/// so it may fail to get the exact value when the expression is complicated.
/// One failing example would be `[$foo].0`
pub(crate) fn eval_cell_path(
    working_set: &StateWorkingSet,
    stack: &Stack,
    head: &Expression,
    path_members: &[PathMember],
    span: Span,
) -> Result<Value, ShellError> {
    // evaluate the head expression to get its value
    let head_value = if let Expr::Var(var_id) = head.expr {
        working_set
            .get_variable(var_id)
            .const_val
            .to_owned()
            .map_or_else(
                || eval_variable(working_set.permanent_state, stack, var_id, span),
                |v| Ok(v),
            )
    } else {
        eval_constant(working_set, head)
    }?;
    head_value.follow_cell_path(path_members, false)
}

fn get_suggestions_by_value(
    value: &Value,
    current_span: reedline::Span,
) -> Vec<SemanticSuggestion> {
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
