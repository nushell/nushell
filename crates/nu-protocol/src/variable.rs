use crate::{Span, Type};

#[derive(Clone, Debug)]
pub struct Variable {
    pub declaration_span: Span,
    pub ty: Type,
    pub mutable: bool,
}

impl Variable {
    pub fn new(declaration_span: Span, ty: Type, mutable: bool) -> Variable {
        Self {
            declaration_span,
            ty,
            mutable,
        }
    }
}
