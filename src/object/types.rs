use std::any::Any;

pub trait Type {
    fn as_any(&self) -> &dyn Any;
    fn equal(&self, other: &dyn Type) -> bool;
}

#[derive(Eq, PartialEq)]
pub struct AnyShell;

impl Type for AnyShell {
    fn as_any(&self) -> &dyn Any {
        self as &dyn Any
    }

    fn equal(&self, other: &dyn Type) -> bool {
        other.as_any().is::<AnyShell>()
    }
}
