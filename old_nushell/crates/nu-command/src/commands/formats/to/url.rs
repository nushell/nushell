use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, UntaggedValue, Value};

pub struct ToUrl;

impl WholeStreamCommand for ToUrl {
    fn name(&self) -> &str {
        "to url"
    }

    fn signature(&self) -> Signature {
        Signature::build("to url")
    }

    fn usage(&self) -> &str {
        "Convert table into url-encoded text"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        to_url(args)
    }
}

fn to_url(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.name_tag();
    let input = args.input;

    Ok(input
        .map(move |value| match value {
            Value {
                value: UntaggedValue::Row(row),
                ..
            } => {
                let mut row_vec = vec![];
                for (k, v) in row.entries {
                    match v.as_string() {
                        Ok(s) => {
                            row_vec.push((k.clone(), s.to_string()));
                        }
                        _ => {
                            return Value::error(ShellError::labeled_error_with_secondary(
                                "Expected table with string values",
                                "requires table with strings",
                                &tag,
                                "value originates from here",
                                v.tag,
                            ));
                        }
                    }
                }

                match serde_urlencoded::to_string(row_vec) {
                    Ok(s) => UntaggedValue::string(s).into_value(&tag),
                    _ => Value::error(ShellError::labeled_error(
                        "Failed to convert to url-encoded",
                        "cannot url-encode",
                        &tag,
                    )),
                }
            }
            Value { tag: value_tag, .. } => Value::error(ShellError::labeled_error_with_secondary(
                "Expected a table from pipeline",
                "requires table input",
                &tag,
                "value originates from here",
                value_tag.span,
            )),
        })
        .into_output_stream())
}

#[cfg(test)]
mod tests {
    use super::ShellError;
    use super::ToUrl;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(ToUrl {})
    }
}
