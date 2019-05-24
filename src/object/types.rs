use std::any::Any;
use std::fmt::Debug;

pub trait Type: Debug + Send {
    fn as_any(&self) -> &dyn Any;
    fn equal(&self, other: &dyn Type) -> bool;
    fn id(&self) -> u64;
    fn copy(&self) -> Box<Type>;
}

#[derive(Debug, Eq, PartialEq)]
pub struct AnyShell;

impl Type for AnyShell {
    fn as_any(&self) -> &dyn Any {
        self as &dyn Any
    }

    fn equal(&self, other: &dyn Type) -> bool {
        other.as_any().is::<AnyShell>()
    }

    fn id(&self) -> u64 {
        0
    }

    fn copy(&self) -> Box<Type> {
        Box::new(AnyShell)
    }
}
