use crate::yaml::Spec;
use derive_setters::Setters;
use nu_protocol::{ShellError, Span, Spanned, Value};

#[non_exhaustive]
#[derive(Debug, Clone, Default, Setters)]
pub struct SerializeOptions {
    spec: Spec,
}

pub fn serialize(value: &Value, options: &SerializeOptions) -> Result<String, ShellError> {
    todo!()
}
