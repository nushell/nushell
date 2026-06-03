use nu_protocol::{Span, Spanned, Value, ShellError};
use crate::yaml::Spec;

#[non_exhaustive]
#[derive(Debug, Clone, Default)]
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