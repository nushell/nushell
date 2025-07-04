use std::borrow::Cow;

use crate::completions::{Completer, CompletionOptions, SemanticSuggestion, SuggestionKind};
use nu_engine::{column::get_columns, eval_variable};
use nu_protocol::{
    ShellError, Span, Value,
    ast::{Expr, Expression, FullCellPath, PathMember},
    engine::{Stack, StateWorkingSet},
    eval_const::eval_constant,
};
use reedline::Suggestion;

use super::completion_options::NuMatcher;

pub struct CellPathCompletion<'a> {
    pub full_cell_path: &'a FullCellPath,
    pub position: usize,
}

fn prefix_from_path_member(member: &PathMember, pos: usize) -> (String, Span) {
    let (prefix_str, start) = match member {
        PathMember::String { val, span, .. } => (val, span.start),
        PathMember::Int { val, span, .. } => (&val.to_string(), span.start),
    };
    let prefix_str = prefix_str.get(..pos + 1 - start).unwrap_or(prefix_str);
    // strip wrapping quotes
    let quotations = ['"', '\'', '`'];
    let prefix_str = prefix_str.strip_prefix(quotations).unwrap_or(prefix_str);
    (prefix_str.to_string(), Span::new(start, pos + 1))
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
        let mut prefix_str = String::new();
        // position at dots, e.g. `$env.config.<TAB>`
        let mut span = Span::new(self.position + 1, self.position + 1);
        let mut path_member_num_before_pos = 0;
        for member in self.full_cell_path.tail.iter() {
            if member.span().end <= self.position {
                path_member_num_before_pos += 1;
            } else if member.span().contains(self.position) {
                (prefix_str, span) = prefix_from_path_member(member, self.position);
                break;
            }
        }

        let current_span = reedline::Span {
            start: span.start - offset,
            end: span.end - offset,
        };

        let mut matcher = NuMatcher::new(prefix_str, options);
        let path_members = self
            .full_cell_path
            .tail
            .get(0..path_member_num_before_pos)
            .unwrap_or_default();
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
                Ok,
            )
    } else {
        eval_constant(working_set, head)
    }?;
    head_value
        .follow_cell_path(path_members)
        .map(Cow::into_owned)
}

fn get_suggestions_by_value(
    value: &Value,
    current_span: reedline::Span,
) -> Vec<SemanticSuggestion> {
    let to_suggestion = |s: String, v: Option<&Value>| {
        // Check if the string needs quoting
        let value = if s.is_empty()
            || s.chars()
                .any(|c: char| !(c.is_ascii_alphabetic() || ['_', '-'].contains(&c)))
        {
            format!("{s:?}")
        } else {
            s
        };

        SemanticSuggestion {
            suggestion: Suggestion {
                value,
                span: current_span,
                description: v.map(|v| v.get_type().to_string()),
                ..Suggestion::default()
            },
            kind: Some(SuggestionKind::CellPath),
        }
    };
    match value {
        Value::Record { val, .. } => val
            .columns()
            .map(|s| to_suggestion(s.to_string(), val.get(s)))
            .collect(),
        Value::List { vals, .. } => get_columns(vals.as_slice())
            .into_iter()
            .map(|s| {
                let sub_val = vals
                    .first()
                    .and_then(|v| v.as_record().ok())
                    .and_then(|rv| rv.get(&s));
                to_suggestion(s, sub_val)
            })
            .collect(),
        _ => vec![],
    }
}
