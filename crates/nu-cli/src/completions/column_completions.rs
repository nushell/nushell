use std::sync::Arc;

use crate::completions::{Completer, CompletionOptions};
use nu_engine::eval_expression_with_input;
use nu_protocol::{
    ast::{Expr, Expression},
    engine::{EngineState, Stack, StateWorkingSet},
    Category, PipelineData, Span, Value,
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
        working_set: &StateWorkingSet,
        prefix: Vec<u8>,
        span: Span,
        offset: usize,
        _: usize,
        options: &CompletionOptions,
    ) -> Vec<Suggestion> {
        let mut input = PipelineData::new(Span::test_data());

        // Skip the last expression
        let index = self.expressions.len().saturating_sub(1);

        // Evaluate previous expressions
        for expr in self.expressions[0..index].iter() {
            if !expr_allowed(&expr.expr, working_set) {
                return vec![];
            }

            // Evaluate expression with input
            input = match eval_expression_with_input(
                &self.engine_state,
                &mut self.stack,
                expr,
                input,
                true,
                false,
            ) {
                Ok(v) => v.0,
                Err(_) => {
                    return vec![];
                }
            };
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
                columns_to_suggestions(cols, prefix, options, span, offset)
            }
            _ => vec![],
        },
        Value::Record { cols, .. } => columns_to_suggestions(cols, prefix, options, span, offset),
        _ => vec![],
    }
}

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

fn expr_allowed(expr: &Expr, working_set: &StateWorkingSet) -> bool {
    log::debug!("expr={:?}", expr);

    match expr {
        Expr::String(..) => true,
        Expr::FullCellPath(cell) => match &cell.head.expr {
            Expr::Var(..) => true,
            Expr::Record(v) => matches!(v.first(), Some((cols,_)) if cols.as_string().is_some()),
            _ => false,
        },
        Expr::Call(call) => {
            let command = working_set.get_decl(call.decl_id);
            let category = command.signature().category;
            let name = command.name();
            let is_custom = command.is_custom_command();

            // Only evaluate expressions without side effects
            if is_custom {
                false
            } else if matches!(category, Category::Core) && name == "echo" {
                true
            } else {
                category_allowed(category)
            }
        }
        _ => false,
    }
}

fn category_allowed(category: Category) -> bool {
    matches!(
        category,
        Category::Bits
            | Category::Bytes
            | Category::Conversions
            | Category::Date
            | Category::Default
            | Category::Filters
            | Category::Formats
            | Category::Generators
            | Category::Hash
            | Category::Math
            | Category::Strings
            | Category::Viewers
    )
}
