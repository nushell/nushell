use crate::completions::{
    Completer, CompletionOptions, MatchAlgorithm, SemanticSuggestion, SuggestionKind,
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
        _prefix: Vec<u8>,
        span: Span,
        offset: usize,
        _pos: usize,
        _options: &CompletionOptions,
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
            ],
            Expr::String(_) => vec![
                ("=~", "Contains regex match"),
                ("!~", "Does not contain regex match"),
                ("in", "In / Contained by (no regex)"),
                (
                    "++",
                    "Appends two lists, a list and a value, two strings, or two binary values",
                ),
                ("not-in", "Not in / Not contained by (no regex)"),
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
                (">=", "Greater than or equal To"),
                ("mod", "Floor division remainder (Modulo)"),
                ("**", "Power of"),
            ],
            Expr::Bool(_) => vec![
                ("and", "Both values are true (w/short-circuit)"),
                ("or", "Either value is true (w/short-circuit)"),
                ("xor", "One value is true and the other is false"),
                ("not", "Negates a value or expression"),
            ],
            Expr::FullCellPath(path) => match path.head.expr {
                Expr::List(_) => vec![(
                    "++",
                    "Appends two lists, a list and a value, two strings, or two binary values",
                )],
                Expr::Var(id) => get_variable_completions(id, working_set),
                _ => vec![],
            },
            _ => vec![],
        };

        let match_algorithm = MatchAlgorithm::Prefix;
        let input_fuzzy_search =
            |(operator, _): &(&str, &str)| match_algorithm.matches_str(operator, partial);

        possible_operations
            .into_iter()
            .filter(input_fuzzy_search)
            .map(move |x| SemanticSuggestion {
                suggestion: Suggestion {
                    value: x.0.to_string(),
                    description: Some(x.1.to_string()),
                    span: reedline::Span::new(span.start - offset, span.end - offset),
                    append_whitespace: true,
                    ..Suggestion::default()
                },
                kind: Some(SuggestionKind::Command(
                    nu_protocol::engine::CommandType::Builtin,
                )),
            })
            .collect()
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
                "Appends a list, a value, a string, or a binary value to a variable.",
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
