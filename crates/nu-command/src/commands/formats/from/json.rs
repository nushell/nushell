use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, UntaggedValue, Value};

#[derive(Debug, thiserror::Error)]
pub enum DeserializationError {
    #[error("Failed to parse input as JSON")]
    Json(#[from] nu_json::Error),

    #[error("Failed to convert JSON to a nushell value")]
    Nu(#[from] Box<nu_serde::Error>),
}

pub struct FromJson;

impl WholeStreamCommand for FromJson {
    fn name(&self) -> &str {
        "from json"
    }

    fn signature(&self) -> Signature {
        Signature::build("from json").switch(
            "objects",
            "treat each line as a separate value",
            Some('o'),
        )
    }

    fn usage(&self) -> &str {
        "Parse text as .json and create table."
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        from_json(args)
    }
}

pub fn from_json_string_to_value(
    s: String,
    tag: impl Into<Tag>,
) -> Result<Value, DeserializationError> {
    let v: nu_json::Value = nu_json::from_str(&s)?;

    Ok(nu_serde::to_value(v, tag).map_err(Box::new)?)
}

fn from_json(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let name_tag = args.call_info.name_tag.clone();

    let objects = args.has_flag("objects");

    let concat_string = args.input.collect_string(name_tag.clone())?;

    if objects {
        #[allow(clippy::needless_collect)]
        let lines: Vec<_> = concat_string.item.lines().map(|x| x.to_string()).collect();
        Ok(lines
            .into_iter()
            .filter_map(move |json_str| {
                if json_str.is_empty() {
                    return None;
                }

                match from_json_string_to_value(json_str, &name_tag) {
                    Ok(x) => Some(x),
                    Err(DeserializationError::Nu(e)) => {
                        let mut message = "Could not convert JSON to nushell value (".to_string();
                        message.push_str(&e.to_string());
                        message.push(')');
                        Some(Value::error(ShellError::labeled_error_with_secondary(
                            message,
                            "input cannot be converted to nushell values",
                            name_tag.clone(),
                            "value originates from here",
                            concat_string.tag.clone(),
                        )))
                    }
                    Err(DeserializationError::Json(e)) => {
                        let mut message = "Could not parse as JSON (".to_string();
                        message.push_str(&e.to_string());
                        message.push(')');

                        Some(Value::error(ShellError::labeled_error_with_secondary(
                            message,
                            "input cannot be parsed as JSON",
                            name_tag.clone(),
                            "value originates from here",
                            concat_string.tag.clone(),
                        )))
                    }
                }
            })
            .into_output_stream())
    } else {
        match from_json_string_to_value(concat_string.item, name_tag.clone()) {
            Ok(x) => match x {
                Value {
                    value: UntaggedValue::Table(list),
                    ..
                } => Ok(list.into_iter().into_output_stream()),

                x => Ok(OutputStream::one(x)),
            },
            Err(DeserializationError::Json(e)) => {
                let mut message = "Could not parse as JSON (".to_string();
                message.push_str(&e.to_string());
                message.push(')');

                Ok(OutputStream::one(Value::error(
                    ShellError::labeled_error_with_secondary(
                        message,
                        "input cannot be parsed as JSON",
                        name_tag,
                        "value originates from here",
                        concat_string.tag,
                    ),
                )))
            }
            Err(DeserializationError::Nu(e)) => {
                let mut message = "Could not convert JSON to nushell value (".to_string();
                message.push_str(&e.to_string());
                message.push(')');
                Ok(OutputStream::one(Value::error(
                    ShellError::labeled_error_with_secondary(
                        message,
                        "input cannot be converted to nushell values",
                        name_tag,
                        "value originates from here",
                        concat_string.tag,
                    ),
                )))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::FromJson;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(FromJson {})
    }
}
