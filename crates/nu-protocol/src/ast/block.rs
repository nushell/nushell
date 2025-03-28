use super::Pipeline;
use crate::{engine::StateWorkingSet, ir::IrBlock, OutDest, Signature, Span, Type, VarId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub signature: Box<Signature>,
    pub pipelines: Vec<Pipeline>,
    pub captures: Vec<(VarId, Span)>,
    pub redirect_env: bool,
    /// The block compiled to IR instructions. Not available for subexpressions.
    pub ir_block: Option<IrBlock>,
    pub span: Option<Span>, // None option encodes no span to avoid using test_span()
}

impl Block {
    pub fn len(&self) -> usize {
        self.pipelines.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pipelines.is_empty()
    }

    pub fn pipe_redirection(
        &self,
        working_set: &StateWorkingSet,
    ) -> (Option<OutDest>, Option<OutDest>) {
        if let Some(first) = self.pipelines.first() {
            first.pipe_redirection(working_set)
        } else {
            (None, None)
        }
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
            ir_block: None,
            span: None,
        }
    }

    pub fn new_with_capacity(capacity: usize) -> Self {
        Self {
            signature: Box::new(Signature::new("")),
            pipelines: Vec::with_capacity(capacity),
            captures: vec![],
            redirect_env: false,
            ir_block: None,
            span: None,
        }
    }

    pub fn output_type(&self) -> Type {
        if let Some(last) = self.pipelines.last() {
            if let Some(last) = last.elements.last() {
                if last.redirection.is_some() {
                    Type::Any
                } else {
                    last.expr.ty.clone()
                }
            } else {
                Type::Nothing
            }
        } else {
            Type::Nothing
        }
    }

    /// Replace any `$in` variables in the initial element of pipelines within the block
    pub fn replace_in_variable(
        &mut self,
        working_set: &mut StateWorkingSet<'_>,
        new_var_id: VarId,
    ) {
        for pipeline in self.pipelines.iter_mut() {
            if let Some(element) = pipeline.elements.first_mut() {
                element.replace_in_variable(working_set, new_var_id);
            }
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
            ir_block: None,
            span: None,
        }
    }
}
