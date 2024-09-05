use crate::completions::{
    completion_common::{adjust_if_intermediate, complete_item, AdjustView},
    Completer, CompletionOptions,
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
    pub fn fetch_int_completions(&self) -> Vec<SemanticSuggestion> {
        println!("int operators");
        vec![]
    }
    pub fn fetch_str_completions(&self) -> Vec<SemanticSuggestion> {
        println!("str operators");
        vec![]
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
        println!("{:?}", self.previous_expr);
        vec![]
    }
}

