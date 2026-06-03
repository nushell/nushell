use crate::yaml::Spec;
use derive_setters::Setters;
use nu_protocol::{ShellError, Span, Spanned, Value};

#[non_exhaustive]
#[derive(Debug, Clone, Default, Setters)]
pub struct ParseOptions {
    keep_styles: bool,
    multiple: ParseMultiple,
    spec: Spec,
}

#[derive(Debug, Clone, Default)]
pub enum ParseMultiple {
    #[default]
    Auto,
    ForceList,
    ForceSingle,
}

pub fn parse(yaml: Spanned<&str>, span: Span, options: &ParseOptions) -> Result<Value, ShellError> {
    todo!()
}
