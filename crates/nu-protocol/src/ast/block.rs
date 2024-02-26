use super::Pipeline;
use crate::{ast::PipelineElement, Signature, Span, Type, VarId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub signature: Box<Signature>,
    pub pipelines: Vec<Pipeline>,
    pub captures: Vec<VarId>,
    pub redirect_env: bool,
    pub span: Option<Span>, // None option encodes no span to avoid using test_span()
}

impl Block {
    pub fn len(&self) -> usize {
        self.pipelines.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pipelines.is_empty()
    }
}

impl Default for Block {
    fn default() -> Self {
        Self::new()
    }
}

impl Block {
    pub fn new() -> Self {
        Self {
            signature: Box::new(Signature::new("")),
            pipelines: vec![],
            captures: vec![],
            redirect_env: false,
            span: None,
        }
    }

    pub fn new_with_capacity(capacity: usize) -> Self {
        Self {
            signature: Box::new(Signature::new("")),
            pipelines: Vec::with_capacity(capacity),
            captures: vec![],
            redirect_env: false,
            span: None,
        }
    }

    pub fn output_type(&self) -> Type {
        if let Some(last) = self.pipelines.last() {
            if let Some(last) = last.elements.last() {
                match last {
                    PipelineElement::Expression(_, expr) => expr.ty.clone(),
                    PipelineElement::ErrPipedExpression(_, expr) => expr.ty.clone(),
                    PipelineElement::OutErrPipedExpression(_, expr) => expr.ty.clone(),
                    PipelineElement::Redirection(_, _, _, _) => Type::Any,
                    PipelineElement::SeparateRedirection { .. } => Type::Any,
                    PipelineElement::SameTargetRedirection { .. } => Type::Any,
                    PipelineElement::And(_, expr) => expr.ty.clone(),
                    PipelineElement::Or(_, expr) => expr.ty.clone(),
                }
            } else {
                Type::Nothing
            }
        } else {
            Type::Nothing
        }
    }
}

impl<T> From<T> for Block
where
    T: Iterator<Item = Pipeline>,
{
    fn from(pipelines: T) -> Self {
        Self {
            signature: Box::new(Signature::new("")),
            pipelines: pipelines.collect(),
            captures: vec![],
            redirect_env: false,
            span: None,
        }
    }
}
