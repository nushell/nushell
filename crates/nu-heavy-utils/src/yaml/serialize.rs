use nu_protocol::{Span, Spanned, Value, ShellError};

use crate::yaml::Spec;

#[non_exhaustive]
#[derive(Debug, Clone, Default)]
pub struct SerializeOptions {
    spec: Spec
}

pub fn serialize(value: &Value, options: &SerializeOptions) -> Result<String, ShellError> {
    todo!()
}