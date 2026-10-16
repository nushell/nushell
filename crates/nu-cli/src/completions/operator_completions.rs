use crate::completions::{
    Completer, Context, Fetched, SemanticSuggestion, completion_options::NuMatcher,
    to_reedline_span,
};
use nu_protocol::{
    ENV_VARIABLE_ID, SuggestionKind, Type, Value,
    ast::{self, Comparison, Expr, Expression},
    engine::StateWorkingSet,
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
                    | Comparison::NotStartsWith
                    | Comparison::EndsWith
                    | Comparison::NotEndsWith
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
    // A cell path's head (`$foo.bar`) and a bare `$foo` left-hand side refer to
    // the same variable, so both count as mutable here.
    let id = match expr {
        Expr::Var(id) => *id,
        Expr::FullCellPath(path) => match path.head.expr {
            Expr::Var(id) => id,
            _ => return false,
        },
        _ => return false,
    };
    if id == ENV_VARIABLE_ID {
        return true;
    }
    let var = working_set.get_variable(id);
    var.mutable
}

impl Completer for OperatorCompletion<'_> {
    fn fetch(&mut self, ctx: &Context) -> Fetched {
        let working_set = ctx.working_set;
        let stack = ctx.stack;
        let span = ctx.span;
        let offset = ctx.offset;
        let options = ctx.options;
        let prefix = ctx.prefix_str();
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
            Type::Any | Type::OneOf(_) => match &self.left_hand_side.expr {
                Expr::FullCellPath(path) => {
                    // for `$ <tab>`
                    if let Expr::Garbage = path.head.expr {
                        return Fetched::answering(vec![]);
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

        let mut matcher = NuMatcher::new(prefix, options, true);
        let suggestion_span = to_reedline_span(span, offset);
        for OperatorItem {
            symbols,
            description,
        } in possible_operations
        {
            matcher.add_semantic_suggestion(SemanticSuggestion {
                suggestion: Suggestion {
                    // Already owned — moved, not cloned.
                    value: symbols,
                    description: Some(description),
                    span: suggestion_span,
                    append_whitespace: true,
                    ..Suggestion::default()
                },
                kind: Some(SuggestionKind::Operator),
            });
        }
        Fetched::answering(matcher.suggestion_results())
    }
}
