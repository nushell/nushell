use crate::completions::{
    Completer, CompletionOptions, SemanticSuggestion, SuggestionKind, completion_options::NuMatcher,
};
use nu_protocol::{
    ENV_VARIABLE_ID, Span, Type, Value,
    ast::{self, Comparison, Expr, Expression},
    engine::{Stack, StateWorkingSet},
};
use reedline::Suggestion;
use strum::{EnumMessage, IntoEnumIterator};

use super::cell_path_completions::eval_cell_path;

#[derive(Clone)]
pub struct OperatorCompletion<'a> {
    pub left_hand_side: &'a Expression,
}

struct OperatorItem {
    pub symbols: String,
    pub description: String,
}

fn operator_to_item<T: EnumMessage + AsRef<str>>(op: T) -> OperatorItem {
    OperatorItem {
        symbols: op.as_ref().into(),
        description: op.get_message().unwrap_or_default().into(),
    }
}

fn common_comparison_ops() -> Vec<OperatorItem> {
    vec![
        operator_to_item(Comparison::In),
        operator_to_item(Comparison::NotIn),
        operator_to_item(Comparison::Equal),
        operator_to_item(Comparison::NotEqual),
    ]
}

fn all_ops_for_immutable() -> Vec<OperatorItem> {
    ast::Comparison::iter()
        .map(operator_to_item)
        .chain(ast::Math::iter().map(operator_to_item))
        .chain(ast::Boolean::iter().map(operator_to_item))
        .chain(ast::Bits::iter().map(operator_to_item))
        .collect()
}

fn collection_comparison_ops() -> Vec<OperatorItem> {
    let mut ops = common_comparison_ops();
    ops.push(operator_to_item(Comparison::Has));
    ops.push(operator_to_item(Comparison::NotHas));
    ops
}

fn number_comparison_ops() -> Vec<OperatorItem> {
    Comparison::iter()
        .filter(|op| {
            !matches!(
                op,
                Comparison::RegexMatch
                    | Comparison::NotRegexMatch
                    | Comparison::StartsWith
                    | Comparison::EndsWith
                    | Comparison::Has
                    | Comparison::NotHas
            )
        })
        .map(operator_to_item)
        .collect()
}

fn math_ops() -> Vec<OperatorItem> {
    ast::Math::iter()
        .filter(|op| !matches!(op, ast::Math::Concatenate | ast::Math::Pow))
        .map(operator_to_item)
        .collect()
}

fn bit_ops() -> Vec<OperatorItem> {
    ast::Bits::iter().map(operator_to_item).collect()
}

fn all_assignment_ops() -> Vec<OperatorItem> {
    ast::Assignment::iter().map(operator_to_item).collect()
}

fn numeric_assignment_ops() -> Vec<OperatorItem> {
    ast::Assignment::iter()
        .filter(|op| !matches!(op, ast::Assignment::ConcatenateAssign))
        .map(operator_to_item)
        .collect()
}

fn concat_assignment_ops() -> Vec<OperatorItem> {
    vec![
        operator_to_item(ast::Assignment::Assign),
        operator_to_item(ast::Assignment::ConcatenateAssign),
    ]
}

fn valid_int_ops() -> Vec<OperatorItem> {
    let mut ops = valid_float_ops();
    ops.extend(bit_ops());
    ops
}

fn valid_float_ops() -> Vec<OperatorItem> {
    let mut ops = valid_value_with_unit_ops();
    ops.push(operator_to_item(ast::Math::Pow));
    ops
}

fn valid_string_ops() -> Vec<OperatorItem> {
    let mut ops: Vec<OperatorItem> = Comparison::iter().map(operator_to_item).collect();
    ops.push(operator_to_item(ast::Math::Concatenate));
    ops.push(OperatorItem {
        symbols: "like".into(),
        description: Comparison::RegexMatch
            .get_message()
            .unwrap_or_default()
            .into(),
    });
    ops.push(OperatorItem {
        symbols: "not-like".into(),
        description: Comparison::NotRegexMatch
            .get_message()
            .unwrap_or_default()
            .into(),
    });
    ops
}

fn valid_list_ops() -> Vec<OperatorItem> {
    let mut ops = collection_comparison_ops();
    ops.push(operator_to_item(ast::Math::Concatenate));
    ops
}

fn valid_binary_ops() -> Vec<OperatorItem> {
    let mut ops = number_comparison_ops();
    ops.extend(bit_ops());
    ops.push(operator_to_item(ast::Math::Concatenate));
    ops
}

fn valid_bool_ops() -> Vec<OperatorItem> {
    let mut ops: Vec<OperatorItem> = ast::Boolean::iter().map(operator_to_item).collect();
    ops.extend(common_comparison_ops());
    ops
}

fn valid_value_with_unit_ops() -> Vec<OperatorItem> {
    let mut ops = number_comparison_ops();
    ops.extend(math_ops());
    ops
}

fn ops_by_value(value: &Value, mutable: bool) -> Vec<OperatorItem> {
    let mut ops = match value {
        Value::Int { .. } => valid_int_ops(),
        Value::Float { .. } => valid_float_ops(),
        Value::String { .. } => valid_string_ops(),
        Value::Binary { .. } => valid_binary_ops(),
        Value::Bool { .. } => valid_bool_ops(),
        Value::Date { .. } => number_comparison_ops(),
        Value::Filesize { .. } | Value::Duration { .. } => valid_value_with_unit_ops(),
        Value::Range { .. } | Value::Record { .. } => collection_comparison_ops(),
        Value::List { .. } => valid_list_ops(),
        _ => all_ops_for_immutable(),
    };
    if mutable {
        ops.extend(match value {
            Value::Int { .. }
            | Value::Float { .. }
            | Value::Filesize { .. }
            | Value::Duration { .. } => numeric_assignment_ops(),
            Value::String { .. } | Value::Binary { .. } | Value::List { .. } => {
                concat_assignment_ops()
            }
            Value::Bool { .. }
            | Value::Date { .. }
            | Value::Range { .. }
            | Value::Record { .. } => vec![operator_to_item(ast::Assignment::Assign)],
            _ => all_assignment_ops(),
        })
    }
    ops
}

fn is_expression_mutable(expr: &Expr, working_set: &StateWorkingSet) -> bool {
    let Expr::FullCellPath(path) = expr else {
        return false;
    };
    let Expr::Var(id) = path.head.expr else {
        return false;
    };
    if id == ENV_VARIABLE_ID {
        return true;
    }
    let var = working_set.get_variable(id);
    var.mutable
}

impl Completer for OperatorCompletion<'_> {
    fn fetch(
        &mut self,
        working_set: &StateWorkingSet,
        stack: &Stack,
        prefix: impl AsRef<str>,
        span: Span,
        offset: usize,
        options: &CompletionOptions,
    ) -> Vec<SemanticSuggestion> {
        let mut needs_assignment_ops = true;
        // Complete according expression type
        // TODO: type inference on self.left_hand_side to get more accurate completions
        let mut possible_operations: Vec<OperatorItem> = match &self.left_hand_side.ty {
            Type::Int | Type::Number => valid_int_ops(),
            Type::Float => valid_float_ops(),
            Type::String => valid_string_ops(),
            Type::Binary => valid_binary_ops(),
            Type::Bool => valid_bool_ops(),
            Type::Date => number_comparison_ops(),
            Type::Filesize | Type::Duration => valid_value_with_unit_ops(),
            Type::Record(_) | Type::Range => collection_comparison_ops(),
            Type::List(_) | Type::Table(_) => valid_list_ops(),
            // Unknown type, resort to evaluated values
            Type::Any => match &self.left_hand_side.expr {
                Expr::FullCellPath(path) => {
                    // for `$ <tab>`
                    if let Expr::Garbage = path.head.expr {
                        return vec![];
                    }
                    let value =
                        eval_cell_path(working_set, stack, &path.head, &path.tail, path.head.span)
                            .unwrap_or_default();
                    let mutable = is_expression_mutable(&self.left_hand_side.expr, working_set);
                    // to avoid duplication
                    needs_assignment_ops = false;
                    ops_by_value(&value, mutable)
                }
                _ => all_ops_for_immutable(),
            },
            _ => common_comparison_ops(),
        };
        // If the left hand side is a variable, add assignment operators if mutable
        if needs_assignment_ops && is_expression_mutable(&self.left_hand_side.expr, working_set) {
            possible_operations.extend(match &self.left_hand_side.ty {
                Type::Int | Type::Float | Type::Number => numeric_assignment_ops(),
                Type::Filesize | Type::Duration => numeric_assignment_ops(),
                Type::String | Type::Binary | Type::List(_) => concat_assignment_ops(),
                Type::Any => all_assignment_ops(),
                _ => vec![operator_to_item(ast::Assignment::Assign)],
            });
        }

        let mut matcher = NuMatcher::new(prefix, options);
        for OperatorItem {
            symbols,
            description,
        } in possible_operations
        {
            matcher.add_semantic_suggestion(SemanticSuggestion {
                suggestion: Suggestion {
                    value: symbols.to_owned(),
                    description: Some(description.to_owned()),
                    span: reedline::Span::new(span.start - offset, span.end - offset),
                    append_whitespace: true,
                    ..Suggestion::default()
                },
                kind: Some(SuggestionKind::Operator),
            });
        }
        matcher.results()
    }
}
