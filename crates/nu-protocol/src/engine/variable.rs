use crate::{FutureSpanId, Type, Value};

#[derive(Clone, Debug)]
pub struct Variable {
    pub declaration_span: FutureSpanId,
    pub ty: Type,
    pub mutable: bool,
    pub const_val: Option<Value>,
}

impl Variable {
    pub fn new(declaration_span: FutureSpanId, ty: Type, mutable: bool) -> Variable {
        Self {
            declaration_span,
            ty,
            mutable,
            const_val: None,
        }
    }
}
