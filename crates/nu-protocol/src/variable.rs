use crate::{Span, Type};

#[derive(Clone, Debug)]
pub struct Variable {
    pub declaration_span: Span,
    pub ty: Type,
}

impl Variable {
    pub fn new(declaration_span: Span, ty: Type) -> Variable {
        Self {
            declaration_span,
            ty,
        }
    }
}
