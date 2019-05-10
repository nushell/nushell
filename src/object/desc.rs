use crate::object::types::{Any, Type};
use derive_new::new;

#[derive(new)]
pub struct DataDescriptor {
    crate name: String,
    crate readonly: bool,
    crate ty: Box<dyn Type>,
}

impl DataDescriptor {
    crate fn any(name: impl Into<String>) -> DataDescriptor {
        DataDescriptor {
            name: name.into(),
            readonly: true,
            ty: Box::new(Any),
        }
    }
}

#[derive(new)]
pub struct DataDescriptorInstance<'desc> {
    desc: &'desc DataDescriptor,
    value: crate::object::base::Value,
}
