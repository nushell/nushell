use crate::{Span, Type, Value};

#[derive(Clone, Debug)]
pub struct Variable {
    pub declaration_span: Span,
    pub ty: Type,
    pub mutable: bool,
    pub const_val: Option<Value>,
    /// The variable's name (e.g. `$foo`), if it was registered by name in a
    /// working-set delta.
    ///
    /// Set when the name→id mapping is inserted into a scope (see
    /// [`StateWorkingSet::insert_variable_into_scope`](crate::engine::StateWorkingSet::insert_variable_into_scope)).
    /// Permanent engine-state variables often leave this as `None` and are still listed by
    /// `scope variables` via permanent overlay name maps. Locals that never reach permanent
    /// overlays rely on this field so stack-based collection can recover their names.
    pub name: Option<Vec<u8>>,
}

impl Variable {
    pub fn new(declaration_span: Span, ty: Type, mutable: bool) -> Variable {
        Self {
            declaration_span,
            ty,
            mutable,
            const_val: None,
            name: None,
        }
    }
}
