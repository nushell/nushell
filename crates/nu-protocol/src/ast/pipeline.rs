use std::ops::{Index, IndexMut};

use crate::{ast::Expression, engine::StateWorkingSet, Span, VarId};

#[derive(Debug, Clone)]
pub enum PipelineElement {
    Expression(Expression),
    Redirect(Expression),
    And(Expression),
    Or(Expression),
}

impl PipelineElement {
    pub fn span(&self) -> Span {
        match self {
            PipelineElement::Expression(expression)
            | PipelineElement::Redirect(expression)
            | PipelineElement::And(expression)
            | PipelineElement::Or(expression) => expression.span,
        }
    }
    pub fn has_in_variable(&self, working_set: &StateWorkingSet) -> bool {
        match self {
            PipelineElement::Expression(expression)
            | PipelineElement::Redirect(expression)
            | PipelineElement::And(expression)
            | PipelineElement::Or(expression) => expression.has_in_variable(working_set),
        }
    }

    pub fn replace_in_variable(&mut self, working_set: &mut StateWorkingSet, new_var_id: VarId) {
        match self {
            PipelineElement::Expression(expression)
            | PipelineElement::Redirect(expression)
            | PipelineElement::And(expression)
            | PipelineElement::Or(expression) => {
                expression.replace_in_variable(working_set, new_var_id)
            }
        }
    }

    pub fn replace_span(
        &mut self,
        working_set: &mut StateWorkingSet,
        replaced: Span,
        new_span: Span,
    ) {
        match self {
            PipelineElement::Expression(expression)
            | PipelineElement::Redirect(expression)
            | PipelineElement::And(expression)
            | PipelineElement::Or(expression) => {
                expression.replace_span(working_set, replaced, new_span)
            }
        }
    }
}

#[derive(Debug, Clone)]
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
                .map(PipelineElement::Expression)
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
