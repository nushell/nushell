use crate::completions::{
    completion_options::NuMatcher, Completer, CompletionOptions, SemanticSuggestion, SuggestionKind,
};
use nu_protocol::{
    ast::{Expr, Expression},
    engine::{Stack, StateWorkingSet},
    Span, Type,
};
use reedline::Suggestion;

#[derive(Clone)]
pub struct OperatorCompletion {
    previous_expr: Expression,
}

impl OperatorCompletion {
    pub fn new(previous_expr: Expression) -> Self {
        OperatorCompletion { previous_expr }
    }
}

impl Completer for OperatorCompletion {
    fn fetch(
        &mut self,
        working_set: &StateWorkingSet,
        _stack: &Stack,
        _prefix: &[u8],
        span: Span,
        offset: usize,
        _pos: usize,
        options: &CompletionOptions,
    ) -> Vec<SemanticSuggestion> {
        //Check if int, float, or string
        let partial = std::str::from_utf8(working_set.get_span_contents(span)).unwrap_or("");
        let op = match &self.previous_expr.expr {
            Expr::BinaryOp(x, _, _) => &x.expr,
            _ => {
                return vec![];
            }
        };
        let possible_operations = match op {
            Expr::Int(_) => vec![
                ("+", "Add (Plus)"),
                ("-", "Subtract (Minus)"),
                ("*", "Multiply"),
                ("/", "Divide"),
                ("==", "Equal to"),
                ("!=", "Not equal to"),
                ("//", "Floor division"),
                ("<", "Less than"),
                (">", "Greater than"),
                ("<=", "Less than or equal to"),
                (">=", "Greater than or equal to"),
                ("mod", "Floor division remainder (Modulo)"),
                ("**", "Power of"),
                ("bit-or", "Bitwise OR"),
                ("bit-xor", "Bitwise exclusive OR"),
                ("bit-and", "Bitwise AND"),
                ("bit-shl", "Bitwise shift left"),
                ("bit-shr", "Bitwise shift right"),
                ("in", "Is a member of (doesn't use regex)"),
                ("not-in", "Is not a member of (doesn't use regex)"),
            ],
            Expr::String(_) => vec![
                ("=~", "Contains regex match"),
                ("like", "Contains regex match"),
                ("!~", "Does not contain regex match"),
                ("not-like", "Does not contain regex match"),
                (
                    "++",
                    "Concatenates two lists, two strings, or two binary values",
                ),
                ("in", "Is a member of (doesn't use regex)"),
                ("not-in", "Is not a member of (doesn't use regex)"),
                ("starts-with", "Starts with"),
                ("ends-with", "Ends with"),
            ],
            Expr::Float(_) => vec![
                ("+", "Add (Plus)"),
                ("-", "Subtract (Minus)"),
                ("*", "Multiply"),
                ("/", "Divide"),
                ("==", "Equal to"),
                ("!=", "Not equal to"),
                ("//", "Floor division"),
                ("<", "Less than"),
                (">", "Greater than"),
                ("<=", "Less than or equal to"),
                (">=", "Greater than or equal to"),
                ("mod", "Floor division remainder (Modulo)"),
                ("**", "Power of"),
                ("in", "Is a member of (doesn't use regex)"),
                ("not-in", "Is not a member of (doesn't use regex)"),
            ],
            Expr::Bool(_) => vec![
                (
                    "and",
                    "Both values are true (short-circuits when first value is false)",
                ),
                (
                    "or",
                    "Either value is true (short-circuits when first value is true)",
                ),
                ("xor", "One value is true and the other is false"),
                ("not", "Negates a value or expression"),
                ("in", "Is a member of (doesn't use regex)"),
                ("not-in", "Is not a member of (doesn't use regex)"),
            ],
            Expr::FullCellPath(path) => match path.head.expr {
                Expr::List(_) => vec![(
                    "++",
                    "Concatenates two lists, two strings, or two binary values",
                )],
                Expr::Var(id) => get_variable_completions(id, working_set),
                _ => vec![],
            },
            _ => vec![],
        };

        let mut matcher = NuMatcher::new(partial, options.clone());
        for (symbol, desc) in possible_operations.into_iter() {
            matcher.add_semantic_suggestion(SemanticSuggestion {
                suggestion: Suggestion {
                    value: symbol.to_string(),
                    description: Some(desc.to_string()),
                    span: reedline::Span::new(span.start - offset, span.end - offset),
                    append_whitespace: true,
                    ..Suggestion::default()
                },
                kind: Some(SuggestionKind::Command(
                    nu_protocol::engine::CommandType::Builtin,
                )),
            });
        }
        matcher.results()
    }
}

pub fn get_variable_completions<'a>(
    id: nu_protocol::Id<nu_protocol::marker::Var>,
    working_set: &StateWorkingSet,
) -> Vec<(&'a str, &'a str)> {
    let var = working_set.get_variable(id);
    if !var.mutable {
        return vec![];
    }

    match var.ty {
        Type::List(_) | Type::String | Type::Binary => vec![
            (
                "++=",
                "Concatenates two lists, two strings, or two binary values",
            ),
            ("=", "Assigns a value to a variable."),
        ],

        Type::Int | Type::Float => vec![
            ("=", "Assigns a value to a variable."),
            ("+=", "Adds a value to a variable."),
            ("-=", "Subtracts a value from a variable."),
            ("*=", "Multiplies a variable by a value"),
            ("/=", "Divides a variable by a value."),
        ],
        _ => vec![],
    }
}
