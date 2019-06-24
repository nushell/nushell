use crate::object::types::Type;
use crate::Text;
use derive_new::new;
use serde::{Deserialize, Serialize, Serializer};

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize, Hash)]
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

    crate fn debug(&self) -> &str {
        match self {
            DescriptorName::String(s) => s,
            DescriptorName::ValueOf => "[[value]]",
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

#[derive(Debug, Deserialize, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, new)]
pub struct DataDescriptor {
    crate name: DescriptorName,
    crate readonly: bool,
    crate ty: Type,
}

impl DataDescriptor {
    crate fn display_header(&self) -> &str {
        self.name.display()
    }

    crate fn is_string_name(&self) -> bool {
        match self.name {
            DescriptorName::String(_) => true,
            DescriptorName::ValueOf => false,
        }
    }
}

impl Serialize for DataDescriptor {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self.name {
            DescriptorName::String(ref s) => serializer.serialize_str(s),
            DescriptorName::ValueOf => serializer.serialize_str("value"),
        }
    }
}

impl From<&str> for DataDescriptor {
    fn from(input: &str) -> DataDescriptor {
        DataDescriptor {
            name: DescriptorName::String(input.to_string()),
            readonly: true,
            ty: Type::Any,
        }
    }
}

impl From<String> for DataDescriptor {
    fn from(input: String) -> DataDescriptor {
        DataDescriptor {
            name: DescriptorName::String(input),
            readonly: true,
            ty: Type::Any,
        }
    }
}

impl From<Text> for DataDescriptor {
    fn from(input: Text) -> DataDescriptor {
        DataDescriptor {
            name: DescriptorName::String(input.to_string()),
            readonly: true,
            ty: Type::Any,
        }
    }
}

impl DescriptorName {
    crate fn for_string_name(name: impl AsRef<str>) -> DescriptorName {
        DescriptorName::String(name.as_ref().into())
    }
}

impl DataDescriptor {
    crate fn value_of() -> DataDescriptor {
        DataDescriptor {
            name: DescriptorName::ValueOf,
            readonly: true,
            ty: Type::Any,
        }
    }

    crate fn for_name(name: impl Into<DescriptorName>) -> DataDescriptor {
        DataDescriptor {
            name: name.into(),
            readonly: true,
            ty: Type::Any,
        }
    }

    crate fn for_string_name(name: impl AsRef<str>) -> DataDescriptor {
        DataDescriptor::for_name(DescriptorName::for_string_name(name))
    }

    crate fn copy(&self) -> DataDescriptor {
        self.clone()
    }
}
