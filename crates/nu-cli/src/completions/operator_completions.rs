use crate::completions::{
    Completer, CompletionOptions, MatchAlgorithm, SemanticSuggestion, SuggestionKind,
};
use nu_protocol::{
    ast::*,
    engine::{Stack, StateWorkingSet},
    Span,
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
        match &self.previous_expr.expr {
            Expr::BinaryOp(x, _, _) => match x.expr {
                Expr::Int(_) => fetch_int_completions(span, offset, partial),
                Expr::String(_) => fetch_str_completions(span, offset, partial),
                Expr::Float(_) => fetch_float_completions(span, offset, partial),
                _ => vec![],
            },
            _ => vec![],
        }
    }
}

pub fn fetch_int_completions(span: Span, offset: usize, partial: &str) -> Vec<SemanticSuggestion> {
    let int_ops = vec![
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
        ("bit-or", "bitwise or"),
        ("bit-xor", "bitwise exclusive or"),
        ("bit-and", "bitwise and"),
        ("bit-shl", "bitwise shift left"),
        ("bit-shr", "bitwise shift right"),
    ];

    let match_algorithm = MatchAlgorithm::Fuzzy;

    let input_fuzzy_search =
        |(operator, _): &(&str, &str)| match_algorithm.matches_str(operator, partial);

    int_ops
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

pub fn fetch_float_completions(
    span: Span,
    offset: usize,
    partial: &str,
) -> Vec<SemanticSuggestion> {
    let float_ops = vec![
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
    ];

    let match_algorithm = MatchAlgorithm::Fuzzy;

    let input_fuzzy_search =
        |(operator, _): &(&str, &str)| match_algorithm.matches_str(operator, partial);

    float_ops
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

pub fn fetch_str_completions(span: Span, offset: usize, partial: &str) -> Vec<SemanticSuggestion> {
    let str_ops = vec![
        ("=~", "Regex Match / Contains"),
        ("!~", "Not Regex Match / Not Contains"),
        ("in", "In / Contains (doesn't use regex)"),
        ("not-in", "Not In / Not Contains (doesn't use regex"),
        ("starts-with", "Starts With"),
        ("ends-with", "Ends With"),
    ];

    let match_algorithm = MatchAlgorithm::Fuzzy;

    let input_fuzzy_search =
        |(operator, _): &(&str, &str)| match_algorithm.matches_str(operator, partial);

    str_ops
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

#[cfg(test)]
mod operator_completion_tests {
    use super::*;

    #[test]
    fn test_int_completions() {
        let span = Span::test_data();
        let offset = 0;
        let dataset = vec![("sh", vec!["bit-shl", "bit-shr"]), ("m", vec!["mod"])];

        for (input, output) in dataset {
            let partial = input;
            let results = fetch_int_completions(span, offset, partial);
            assert_eq!(results.len(), output.len());
            results
                .into_iter()
                .map(|x| x.suggestion.value.clone())
                .zip(output.into_iter())
                .for_each(|(result, expected)| assert_eq!(result.as_str(), expected));
        }
    }

    #[test]
    fn test_float_completions() {
        let span = Span::test_data();
        let offset = 0;
        let dataset = vec![("sh", vec![]), ("m", vec!["mod"])];

        for (input, output) in dataset {
            let partial = input;
            let results = fetch_float_completions(span, offset, partial);
            assert_eq!(results.len(), output.len());
            results
                .into_iter()
                .map(|x| x.suggestion.value.clone())
                .zip(output.into_iter())
                .for_each(|(result, expected)| assert_eq!(result.as_str(), expected));
        }
    }

    #[test]
    fn test_str_completions() {
        let span = Span::test_data();
        let offset = 0;
        let dataset = vec![("s", vec!["starts-with", "ends-with"])];

        for (input, output) in dataset {
            let partial = input;
            let results = fetch_str_completions(span, offset, partial);
            assert_eq!(results.len(), output.len());
            results
                .into_iter()
                .map(|x| x.suggestion.value.clone())
                .zip(output.into_iter())
                .for_each(|(result, expected)| assert_eq!(result.as_str(), expected));
        }
    }
}
