use crate::{ast::Expression, engine::StateWorkingSet, Span};
use serde::{Deserialize, Serialize};
use std::ops::{Index, IndexMut};

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub enum Redirection {
    Stdout,
    Stderr,
    StdoutAndStderr,
}

// Note: Span in the below is for the span of the connector not the whole element
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PipelineElement {
    Expression(Option<Span>, Expression),
    ErrPipedExpression(Option<Span>, Expression),
    OutErrPipedExpression(Option<Span>, Expression),
    // final field indicates if it's in append mode
    Redirection(Span, Redirection, Expression, bool),
    // final bool field indicates if it's in append mode
    SeparateRedirection {
        out: (Span, Expression, bool),
        err: (Span, Expression, bool),
    },
    // redirection's final bool field indicates if it's in append mode
    SameTargetRedirection {
        cmd: (Option<Span>, Expression),
        redirection: (Span, Expression, bool),
    },
    And(Span, Expression),
    Or(Span, Expression),
}

impl PipelineElement {
    pub fn expression(&self) -> &Expression {
        match self {
            PipelineElement::Expression(_, expression)
            | PipelineElement::ErrPipedExpression(_, expression)
            | PipelineElement::OutErrPipedExpression(_, expression) => expression,
            PipelineElement::Redirection(_, _, expression, _) => expression,
            PipelineElement::SeparateRedirection {
                out: (_, expression, _),
                ..
            } => expression,
            PipelineElement::SameTargetRedirection {
                cmd: (_, expression),
                ..
            } => expression,
            PipelineElement::And(_, expression) => expression,
            PipelineElement::Or(_, expression) => expression,
        }
    }

    pub fn span(&self) -> Span {
        match self {
            PipelineElement::Expression(None, expression)
            | PipelineElement::ErrPipedExpression(None, expression)
            | PipelineElement::OutErrPipedExpression(None, expression)
            | PipelineElement::SameTargetRedirection {
                cmd: (None, expression),
                ..
            } => expression.span,
            PipelineElement::Expression(Some(span), expression)
            | PipelineElement::ErrPipedExpression(Some(span), expression)
            | PipelineElement::OutErrPipedExpression(Some(span), expression)
            | PipelineElement::Redirection(span, _, expression, _)
            | PipelineElement::SeparateRedirection {
                out: (span, expression, _),
                ..
            }
            | PipelineElement::And(span, expression)
            | PipelineElement::Or(span, expression)
            | PipelineElement::SameTargetRedirection {
                cmd: (Some(span), expression),
                ..
            } => Span {
                start: span.start,
                end: expression.span.end,
            },
        }
    }
    pub fn has_in_variable(&self, working_set: &StateWorkingSet) -> bool {
        match self {
            PipelineElement::Expression(_, expression)
            | PipelineElement::ErrPipedExpression(_, expression)
            | PipelineElement::OutErrPipedExpression(_, expression)
            | PipelineElement::Redirection(_, _, expression, _)
            | PipelineElement::And(_, expression)
            | PipelineElement::Or(_, expression)
            | PipelineElement::SameTargetRedirection {
                cmd: (_, expression),
                ..
            } => expression.has_in_variable(working_set),
            PipelineElement::SeparateRedirection {
                out: (_, out_expr, _),
                err: (_, err_expr, _),
            } => out_expr.has_in_variable(working_set) || err_expr.has_in_variable(working_set),
        }
    }

    pub fn replace_span(
        &mut self,
        working_set: &mut StateWorkingSet,
        replaced: Span,
        new_span: Span,
    ) {
        match self {
            PipelineElement::Expression(_, expression)
            | PipelineElement::ErrPipedExpression(_, expression)
            | PipelineElement::OutErrPipedExpression(_, expression)
            | PipelineElement::Redirection(_, _, expression, _)
            | PipelineElement::And(_, expression)
            | PipelineElement::Or(_, expression)
            | PipelineElement::SameTargetRedirection {
                cmd: (_, expression),
                ..
            }
            | PipelineElement::SeparateRedirection {
                out: (_, expression, _),
                ..
            } => expression.replace_span(working_set, replaced, new_span),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pipeline {
    pub elements: Vec<PipelineElement>,
}

impl Default for Pipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl Pipeline {
    pub fn new() -> Self {
        Self { elements: vec![] }
    }

    pub fn from_vec(expressions: Vec<Expression>) -> Pipeline {
        Self {
            elements: expressions
                .into_iter()
                .enumerate()
                .map(|(idx, x)| {
                    PipelineElement::Expression(if idx == 0 { None } else { Some(x.span) }, x)
                })
                .collect(),
        }
    }

    pub fn len(&self) -> usize {
        self.elements.len()
    }

    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }
}

impl Index<usize> for Pipeline {
    type Output = PipelineElement;

    fn index(&self, index: usize) -> &Self::Output {
        &self.elements[index]
    }
}

impl IndexMut<usize> for Pipeline {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.elements[index]
    }
}
