use std::borrow::Cow;

use crate::completions::{Completer, Context, Fetched, SemanticSuggestion, to_reedline_span};
use nu_engine::{column::get_columns, eval_variable};
use nu_protocol::{
    ShellError, Span, SuggestionKind, Type, Value,
    ast::{Expr, Expression, FullCellPath, PathMember},
    engine::{Stack, StateWorkingSet},
    eval_const::eval_constant,
};
use reedline::Suggestion;

use super::completion_options::NuMatcher;

pub struct CellPathCompletion<'a> {
    pub full_cell_path: &'a FullCellPath,
    /// The cursor, in absolute working-set (span) coordinates.
    pub cursor: usize,
}

/// The typed portion of `member` up to the cursor, and the span that portion occupies.
fn prefix_from_path_member(member: &PathMember, cursor: usize) -> (String, Span) {
    let (val, start) = match member {
        PathMember::String { val, span, .. } => (val, span.start),
        PathMember::Int { val, span, .. } => (&val.to_string(), span.start),
    };
    let prefix_str = val.get(..cursor - start).unwrap_or(val);
    (prefix_str.to_string(), Span::new(start, cursor))
}

impl Completer for CellPathCompletion<'_> {
    fn fetch(&mut self, ctx: &Context) -> Fetched {
        let working_set = ctx.working_set;
        let stack = ctx.stack;
        let offset = ctx.offset;
        let options = ctx.options;
        let mut prefix_str = String::new();
        let cursor = self.cursor;
        // Completing at a dot with no partial member yet: empty span at the cursor.
        let mut span = Span::new(cursor, cursor);
        let mut path_member_num_before_pos = 0;
        for member in self.full_cell_path.tail.iter() {
            if member.span().end < cursor {
                path_member_num_before_pos += 1;
            } else if member.span().contains(cursor) || member.span().end == cursor {
                (prefix_str, span) = prefix_from_path_member(member, cursor);
                break;
            }
        }

        let current_span = to_reedline_span(span, offset);

        let mut matcher = NuMatcher::new(prefix_str, options, true);
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
        );

        if let Ok(value) = value {
            for suggestion in get_suggestions_by_value(&value, current_span) {
                matcher.add_semantic_suggestion(suggestion);
            }
        } else if let Some(ty) = self.full_cell_path.head.ty.follow_cell_path(path_members) {
            for suggestion in get_suggestions_by_type(&ty, current_span) {
                matcher.add_semantic_suggestion(suggestion);
            }
        }

        Fetched::answering(matcher.suggestion_results())
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
                // Handles `$env` / `$nu` / `$ans` specials as well as stack vars.
                // Read `$ans` without deferring truncation warnings (completions are not
                // a user-facing display of the value).
                || {
                    if var_id == nu_protocol::LAST_VARIABLE_ID {
                        stack.get_var(var_id, span)
                    } else {
                        eval_variable(working_set.permanent_state, stack, var_id, span)
                    }
                },
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
        Value::Custom { val, .. } => match val.type_name().as_str() {
            "semver" => ["major", "minor", "patch", "pre", "build", "prefix"]
                .into_iter()
                .map(|s| to_suggestion(s.to_string(), None))
                .collect(),
            "matrix" => ["shape", "ndim", "size"]
                .into_iter()
                .map(|s| to_suggestion(s.to_string(), None))
                .collect(),
            _ => vec![],
        },
        _ => vec![],
    }
}

fn get_suggestions_by_type(ty: &Type, current_span: reedline::Span) -> Vec<SemanticSuggestion> {
    match ty {
        Type::Record(columns) | Type::Table(columns) => columns
            .iter()
            .map(|(name, ty)| SemanticSuggestion {
                suggestion: Suggestion {
                    value: name.to_string(),
                    span: current_span,
                    description: Some(ty.to_string()),
                    ..Suggestion::default()
                },
                kind: Some(SuggestionKind::CellPath),
            })
            .collect(),
        Type::List(inner) => get_suggestions_by_type(inner, current_span),
        _ => vec![],
    }
}
