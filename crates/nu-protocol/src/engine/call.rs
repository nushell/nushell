use crate::{ast, ir, Span};

/// This is a HACK to help [`Command`](super::Command) support both the old AST evaluator and the
/// new IR evaluator at the same time. It should be removed once we are satisfied with the new
/// evaluator.
#[derive(Debug, Clone)]
pub struct Call<'a> {
    pub head: Span,
    inner: CallImpl<'a>,
}

#[derive(Debug, Clone)]
enum CallImpl<'a> {
    Ast(&'a ast::Call),
    Ir(ir::Call<'a>),
}
