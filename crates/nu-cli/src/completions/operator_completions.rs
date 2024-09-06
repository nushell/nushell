use crate::completions::{
    completion_common::{adjust_if_intermediate, complete_item, AdjustView},
    Completer, CompletionOptions, SuggestionKind, MatchAlgorithm
};
use nu_ansi_term::Style;
use nu_protocol::{
    engine::{EngineState, Stack, StateWorkingSet},
    debugger::WithoutDebug,
    Span,
    ast::*
};
use nu_engine::{
    eval_expression
};
use nu_utils::IgnoreCaseExt;
use reedline::Suggestion;
use std::path::Path;

use super::SemanticSuggestion;

#[derive(Clone)]
pub struct OperatorCompletion{
    previous_expr: Expression
}

impl OperatorCompletion {
    pub fn new(previous_expr: Expression) -> Self {
        OperatorCompletion {
            previous_expr
        }
    }
}


impl OperatorCompletion {
    pub fn fetch_int_completions(&self, span: Span, offset: usize, partial: &str) -> Vec<SemanticSuggestion> {
        let int_ops = vec![
            ("mod", "Modulo"),
            ("bit-or", "bitwise or"), 
            ("bit-xor", "bitwise exclusive or"),
            ("bit-and", "bitwise and"),
            ("bit-shl", "bitwise shift left"),
            ("bit-shr", "bitwise shift right")
        ];

        let match_algorithm = MatchAlgorithm::Fuzzy;

        let input_fuzzy_search = |(operator, _): &(&str, &str)| match_algorithm.matches_str(operator, partial);

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
                kind: Some(SuggestionKind::Command(nu_protocol::engine::CommandType::Builtin)),
            })
            .collect()

    }
    pub fn fetch_str_completions(&self, span: Span, offset: usize, partial: &str) -> Vec<SemanticSuggestion> {
        let str_ops = vec![
            ("in" , "In / Contains (doesn't use regex)"),
            ("not-in" , "Not In / Not Contains (doesn't use regex"),
            ("starts-with" , "Starts With"),
            ("ends-with", "Ends With")
        ];

        let match_algorithm = MatchAlgorithm::Fuzzy;

        let input_fuzzy_search = |(operator, _): &(&str, &str)| match_algorithm.matches_str(operator, partial);

        str_ops
            .into_iter()
            .map(move |x| SemanticSuggestion {
                suggestion: Suggestion {
                    value: x.0.to_string(),
                    description: Some(x.1.to_string()),
                    span: reedline::Span::new(span.start - offset, span.end - offset),
                    append_whitespace: true,
                    ..Suggestion::default()
                },
                kind: Some(SuggestionKind::Command(nu_protocol::engine::CommandType::Builtin)),
            })
            .collect()
    }
    
}

impl Completer for OperatorCompletion {
    fn fetch(
        &mut self,
        working_set: &StateWorkingSet,
        stack: &Stack,
        prefix: Vec<u8>,
        span: Span,
        offset: usize,
        _pos: usize,
        options: &CompletionOptions,
    ) -> Vec<SemanticSuggestion> {
        //Check if int, numeric, or string
        let partial = std::str::from_utf8(working_set.get_span_contents(span)).unwrap_or("");
        match &self.previous_expr.expr {
            Expr::BinaryOp(x, _, _) => {
                match x.expr {
                    Expr::Int(_) => self.fetch_int_completions(span, offset, partial), 
                    Expr::String(_) => self.fetch_str_completions(span, offset, partial), 
                    _ => vec![]
                }
            },
            _ => vec![]
            
        }
    }
}

