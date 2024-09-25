use crate::completions::{
    Completer, CompletionOptions, MatchAlgorithm, SemanticSuggestion, SuggestionKind,
};
use nu_parser::FlatShape;
use nu_protocol::{
    engine::{Stack, StateWorkingSet},
    Span,
};
use reedline::Suggestion;

#[derive(Clone)]
pub struct OperatorCompletion {
    previous_expr_shape: FlatShape,
}

impl OperatorCompletion {
    pub fn new(previous_expr_shape: FlatShape) -> Self {
        OperatorCompletion {
            previous_expr_shape,
        }
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
        let possible_operations = match &self.previous_expr_shape {
            FlatShape::Int => vec![
                ("+", "Plus / Addition"),
                ("-", "Minus / Subtraction"),
                ("*", "Multiply"),
                ("/", "Divide"),
                ("==", "Equal"),
                ("!=", "Not Equal"),
                ("//", "Floor Division"),
                ("<", "Less Than"),
                (">", "Greater Than"),
                ("<=", "Less Than or Equal to"),
                (">=", "Greater Than or Equal to"),
                ("mod", "Modulo"),
                ("**", "Pow"),
                ("bit-or", "bitwise or"),
                ("bit-xor", "bitwise exclusive or"),
                ("bit-and", "bitwise and"),
                ("bit-shl", "bitwise shift left"),
                ("bit-shr", "bitwise shift right"),
            ],
            FlatShape::String => vec![
                ("=~", "Regex Match / Contains"),
                ("!~", "Not Regex Match / Not Contains"),
                ("in", "In / Contains (doesn't use regex)"),
                (
                    "++",
                    "Appends two lists, a list and a value, two strings, or two binary values",
                ),
                ("not-in", "Not In / Not Contains (doesn't use regex"),
                ("starts-with", "Starts With"),
                ("ends-with", "Ends With"),
            ],
            FlatShape::Float => vec![
                ("+", "Plus / Addition"),
                ("-", "Minus / Subtraction"),
                ("*", "Multiply"),
                ("/", "Divide"),
                ("=", "Equal"),
                ("!=", "Not Equal"),
                ("//", "Floor Division"),
                ("<", "Less Than"),
                (">", "Greater Than"),
                ("<=", "Less Than or Equal to"),
                (">=", "Greater Than or Equal to"),
                ("mod", "Modulo"),
                ("**", "Pow"),
            ],
            FlatShape::Bool => vec![
                ("and", "Checks if both values are true."),
                ("or", "Checks if either value is true."),
                ("xor", "Checks if one value is true and the other is false."),
                ("not", "Negates a value or expression."),
            ],
            FlatShape::List => vec![(
                "++",
                "Appends two lists, a list and a value, two strings, or two binary values",
            )],
            FlatShape::Variable(_) => vec![
                ("=", "Assigns a value to a variable."),
                ("+=", "Adds a value to a variable."),
                (
                    "++=",
                    "Appends a list, a value, a string, or a binary value to a variable.",
                ),
                ("-=", "Subtracts a value from a variable."),
                ("*=", "Multiplies a variable by a value"),
                ("/=", "Divides a variable by a value."),
            ],
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
