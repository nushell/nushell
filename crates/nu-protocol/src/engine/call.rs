use crate::{
    ast,
    ir::{self, Instruction},
    DeclId, Span,
};

/// This is a HACK to help [`Command`](super::Command) support both the old AST evaluator and the
/// new IR evaluator at the same time. It should be removed once we are satisfied with the new
/// evaluator.
#[derive(Debug, Clone)]
pub struct Call<'a> {
    pub head: Span,
    pub decl_id: DeclId,
    pub inner: CallImpl<'a>,
}

#[derive(Debug, Clone)]
pub enum CallImpl<'a> {
    Ast(&'a ast::Call),
    Ir(ir::Call<'a>),
}

impl<'a> From<&'a ast::Call> for Call<'a> {
    fn from(call: &'a ast::Call) -> Self {
        Call {
            head: call.head,
            decl_id: call.decl_id,
            inner: CallImpl::Ast(call),
        }
    }
}

impl<'a> From<ir::Call<'a>> for Call<'a> {
    fn from(call: ir::Call<'a>) -> Self {
        let Instruction::Call { decl_id, .. } = *call.instruction else {
            panic!("ir::Call instruction was not Call: {:?}", call.instruction);
        };
        Call {
            head: *call.head,
            decl_id,
            inner: CallImpl::Ir(call),
        }
    }
}
