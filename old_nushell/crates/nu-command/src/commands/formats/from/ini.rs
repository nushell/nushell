use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, UntaggedValue, Value};
use std::collections::HashMap;

#[derive(Debug, thiserror::Error)]
pub enum DeserializationError {
    #[error("Failed to parse input as INI")]
    Ini(#[from] serde_ini::de::Error),

    #[error("Failed to convert to a nushell value")]
    Nu(#[from] nu_serde::Error),
}

pub struct FromIni;

impl WholeStreamCommand for FromIni {
    fn name(&self) -> &str {
        "from ini"
    }

    fn signature(&self) -> Signature {
        Signature::build("from ini")
    }

    fn usage(&self) -> &str {
        "Parse text as .ini and create table"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        from_ini(args)
    }
}

pub fn from_ini_string_to_value(
    s: String,
    tag: impl Into<Tag>,
) -> Result<Value, DeserializationError> {
    let v: HashMap<String, HashMap<String, String>> = serde_ini::from_str(&s)?;

    Ok(nu_serde::to_value(v, tag)?)
}

fn from_ini(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.name_tag();
    let input = args.input;
    let concat_string = input.collect_string(tag.clone())?;

    match from_ini_string_to_value(concat_string.item, tag.clone()) {
        Ok(x) => match x {
            Value {
                value: UntaggedValue::Table(list),
                ..
            } => Ok(list.into_iter().into_output_stream()),
            x => Ok(OutputStream::one(x)),
        },
        Err(DeserializationError::Ini(e)) => Err(ShellError::labeled_error_with_secondary(
            format!("Could not parse as INI: {}", e),
            "input cannot be parsed as INI",
            &tag,
            "value originates from here",
            concat_string.tag,
        )),
        Err(DeserializationError::Nu(e)) => Err(ShellError::labeled_error_with_secondary(
            format!("Could not convert to nushell value: {}", e),
            "input cannot be converted to nushell",
            &tag,
            "value originates from here",
            concat_string.tag,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::FromIni;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(FromIni {})
    }
}
