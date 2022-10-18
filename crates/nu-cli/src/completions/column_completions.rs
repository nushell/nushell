use std::sync::Arc;

use crate::completions::{Completer, CompletionOptions};
use nu_engine::{eval_expression, eval_expression_with_input};
use nu_protocol::{
    ast::Expression,
    engine::{EngineState, Stack, StateWorkingSet},
    IntoPipelineData, PipelineData, Span, Value,
};

use reedline::Suggestion;

#[derive(Clone)]
pub struct ColumnCompletion {
    expressions: Vec<Expression>,
    engine_state: Arc<EngineState>,
    stack: Stack,
}

impl ColumnCompletion {
    pub fn new(expressions: Vec<Expression>, engine_state: Arc<EngineState>, stack: Stack) -> Self {
        Self {
            expressions,
            engine_state,
            stack,
        }
    }
}

impl Completer for ColumnCompletion {
    fn fetch(
        &mut self,
        _working_set: &StateWorkingSet,
        prefix: Vec<u8>,
        span: Span,
        offset: usize,
        _: usize,
        options: &CompletionOptions,
    ) -> Vec<Suggestion> {
        let mut input: PipelineData = PipelineData::new(Span::test_data());

        let max_index = self.expressions.len().saturating_sub(1);

        // Evaluate previous expressions
        for (index, expr) in self.expressions.iter().enumerate() {
            // Skip the last expression
            if index == max_index {
                break;
            }

            // Evaluate first expression without input
            if index == 0 {
                input = match eval_expression(&self.engine_state, &mut self.stack, expr) {
                    Ok(v) => v.into_pipeline_data(),
                    Err(_) => return vec![],
                };
            } else {
                // Evaluate the other expressions with input
                input = match eval_expression_with_input(
                    &self.engine_state,
                    &mut self.stack,
                    expr,
                    input,
                    true,
                    false,
                ) {
                    Ok((data, _)) => data,
                    Err(_) => return vec![],
                }
            }
        }

        match input {
            PipelineData::Value(value, ..) => {
                value_to_suggestions(&value, &prefix, options, span, offset)
            }
            PipelineData::ListStream(mut stream, ..) => match stream.next() {
                Some(value) => value_to_suggestions(&value, &prefix, options, span, offset),
                _ => vec![],
            },
            _ => {
                vec![]
            }
        }
    }
}

// Convert value to suggestions
fn value_to_suggestions(
    value: &Value,
    prefix: &[u8],
    options: &CompletionOptions,
    span: Span,
    offset: usize,
) -> Vec<Suggestion> {
    match value {
        Value::List { vals, .. } => match vals.first() {
            Some(Value::Record { cols, .. }) => {
                columns_to_suggestions(cols, &prefix, options, span, offset)
            }
            _ => vec![],
        },
        Value::Record { cols, .. } => columns_to_suggestions(&cols, &prefix, options, span, offset),
        _ => vec![],
    }
}

// Convert the columns to suggestions
fn columns_to_suggestions(
    columns: &[String],
    prefix: &[u8],
    options: &CompletionOptions,
    span: Span,
    offset: usize,
) -> Vec<Suggestion> {
    columns
        .iter()
        .filter(|s| options.match_algorithm.matches_u8(s.as_bytes(), prefix))
        .map(|s| Suggestion {
            value: s.to_owned(),
            description: None,
            extra: None,
            span: reedline::Span {
                start: span.start - offset,
                end: span.end - offset,
            },
            append_whitespace: false,
        })
        .collect()
}
