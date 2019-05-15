use crate::object::types::Type;
use derive_new::new;

#[derive(new)]
pub struct DataDescriptor {
    crate name: String,
    crate readonly: bool,
    crate ty: Box<dyn Type>,
}

impl PartialEq for DataDescriptor {
    fn eq(&self, other: &DataDescriptor) -> bool {
        self.name == other.name && self.readonly == other.readonly && self.ty.equal(&*other.ty)
    }
}

impl DataDescriptor {}
