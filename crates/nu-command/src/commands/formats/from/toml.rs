use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, UntaggedValue, Value};

#[derive(Debug, thiserror::Error)]
pub enum DeserializationError {
    #[error("Failed to parse input as TOML")]
    Toml(#[from] toml::de::Error),

    #[error("Failed to convert to a nushell value")]
    Nu(#[from] Box<nu_serde::Error>),
}

pub struct FromToml;

impl WholeStreamCommand for FromToml {
    fn name(&self) -> &str {
        "from toml"
    }

    fn signature(&self) -> Signature {
        Signature::build("from toml")
    }

    fn usage(&self) -> &str {
        "Parse text as .toml and create table."
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        from_toml(args)
    }
}

pub fn from_toml_string_to_value(
    s: String,
    tag: impl Into<Tag>,
) -> Result<Value, DeserializationError> {
    let v: toml::Value = s.parse::<toml::Value>()?;

    Ok(nu_serde::to_value(v, tag).map_err(Box::new)?)
}

pub fn from_toml(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.name_tag();
    let input = args.input;

    let concat_string = input.collect_string(tag.clone())?;
    Ok(
        match from_toml_string_to_value(concat_string.item, tag.clone()) {
            Ok(x) => match x {
                Value {
                    value: UntaggedValue::Table(list),
                    ..
                } => list.into_iter().into_output_stream(),
                x => OutputStream::one(x),
            },
            Err(_) => {
                return Err(ShellError::labeled_error_with_secondary(
                    "Could not parse as TOML",
                    "input cannot be parsed as TOML",
                    &tag,
                    "value originates from here",
                    concat_string.tag,
                ))
            }
        },
    )
}

#[cfg(test)]
mod tests {
    use super::FromToml;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(FromToml {})
    }
}
