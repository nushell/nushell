use std::sync::Arc;

use crate::{ast, ir, DeclId, Span};

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
    AstRef(&'a ast::Call),
    AstArc(Arc<ast::Call>),
    IrRef(&'a ir::Call),
    IrArc(Arc<ir::Call>),
}

impl Call<'_> {
    pub fn to_owned(&self) -> Call<'static> {
        Call {
            head: self.head,
            decl_id: self.decl_id,
            inner: self.inner.to_owned(),
        }
    }
}

impl CallImpl<'_> {
    pub fn to_owned(&self) -> CallImpl<'static> {
        match self {
            CallImpl::AstRef(call) => CallImpl::AstArc(Arc::new((*call).clone())),
            CallImpl::AstArc(call) => CallImpl::AstArc(call.clone()),
            CallImpl::IrRef(call) => CallImpl::IrArc(Arc::new((*call).clone())),
            CallImpl::IrArc(call) => CallImpl::IrArc(call.clone()),
        }
    }
}

impl<'a> From<&'a ast::Call> for Call<'a> {
    fn from(call: &'a ast::Call) -> Self {
        Call {
            head: call.head,
            decl_id: call.decl_id,
            inner: CallImpl::AstRef(call),
        }
    }
}

impl<'a> From<&'a ir::Call> for Call<'a> {
    fn from(call: &'a ir::Call) -> Self {
        Call {
            head: call.head,
            decl_id: call.decl_id,
            inner: CallImpl::IrRef(call),
        }
    }
}
