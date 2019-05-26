use crate::object::types::{AnyShell, Type};
use derive_new::new;

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum DescriptorName {
    String(String),
    ValueOf,
}

impl DescriptorName {
    crate fn display(&self) -> &str {
        match self {
            DescriptorName::String(s) => s,
            DescriptorName::ValueOf => "value",
        }
    }

    crate fn as_string(&self) -> Option<&str> {
        match self {
            DescriptorName::String(s) => Some(s),
            DescriptorName::ValueOf => None,
        }
    }

    crate fn is_string(&self, string: &str) -> bool {
        match self {
            DescriptorName::String(s) => s == string,
            DescriptorName::ValueOf => false,
        }
    }
}

#[derive(Debug, new)]
pub struct DataDescriptor {
    crate name: DescriptorName,
    crate readonly: bool,
    crate ty: Box<dyn Type>,
}

impl From<&str> for DataDescriptor {
    fn from(input: &str) -> DataDescriptor {
        DataDescriptor {
            name: DescriptorName::String(input.to_string()),
            readonly: true,
            ty: Box::new(AnyShell),
        }
    }
}

impl From<String> for DataDescriptor {
    fn from(input: String) -> DataDescriptor {
        DataDescriptor {
            name: DescriptorName::String(input),
            readonly: true,
            ty: Box::new(AnyShell),
        }
    }
}

impl PartialEq for DataDescriptor {
    fn eq(&self, other: &DataDescriptor) -> bool {
        self.name == other.name && self.readonly == other.readonly && self.ty.equal(&*other.ty)
    }
}

impl std::hash::Hash for DataDescriptor {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        self.readonly.hash(state);
        self.ty.id().hash(state);
    }
}

impl Eq for DataDescriptor {}

impl DescriptorName {
    crate fn for_string_name(name: impl Into<String>) -> DescriptorName {
        DescriptorName::String(name.into())
    }
}

impl Clone for DataDescriptor {
    fn clone(&self) -> DataDescriptor {
        DataDescriptor {
            name: self.name.clone(),
            readonly: self.readonly,
            ty: self.ty.copy(),
        }
    }
}

impl DataDescriptor {
    crate fn value_of() -> DataDescriptor {
        DataDescriptor {
            name: DescriptorName::ValueOf,
            readonly: true,
            ty: Box::new(AnyShell),
        }
    }

    crate fn for_name(name: impl Into<DescriptorName>) -> DataDescriptor {
        DataDescriptor {
            name: name.into(),
            readonly: true,
            ty: Box::new(AnyShell),
        }
    }

    crate fn for_string_name(name: impl Into<String>) -> DataDescriptor {
        DataDescriptor::for_name(DescriptorName::for_string_name(name))
    }

    crate fn copy(&self) -> DataDescriptor {
        DataDescriptor {
            name: self.name.clone(),
            readonly: self.readonly,
            ty: self.ty.copy(),
        }
    }
}
