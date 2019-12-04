use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, UntaggedValue, Value};

pub struct ToURL;

impl WholeStreamCommand for ToURL {
    fn name(&self) -> &str {
        "to-url"
    }

    fn signature(&self) -> Signature {
        Signature::build("to-url")
    }

    fn usage(&self) -> &str {
        "Convert table into url-encoded text"
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        to_url(args, registry)
    }
}

fn to_url(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry)?;
    let tag = args.name_tag();
    let input = args.input;

    let stream = async_stream! {
        let input: Vec<Value> = input.values.collect().await;

        for value in input {
            match value {
                Value { value: UntaggedValue::Row(row), .. } => {
                    let mut row_vec = vec![];
                    for (k,v) in row.entries {
                        match v.as_string() {
                            Ok(s) => {
                                row_vec.push((k.clone(), s.to_string()));
                            }
                            _ => {
                                yield Err(ShellError::labeled_error_with_secondary(
                                    "Expected table with string values",
                                    "requires table with strings",
                                    &tag,
                                    "value originates from here",
                                    v.tag,
                                ))
                            }
                        }
                    }

                    match serde_urlencoded::to_string(row_vec) {
                        Ok(s) => {
                            yield ReturnSuccess::value(UntaggedValue::string(s).into_value(&tag));
                        }
                        _ => {
                            yield Err(ShellError::labeled_error(
                                "Failed to convert to url-encoded",
                                "cannot url-encode",
                                &tag,
                            ))
                        }
                    }
                }
                Value { tag: value_tag, .. } => {
                    yield Err(ShellError::labeled_error_with_secondary(
                        "Expected a table from pipeline",
                        "requires table input",
                        &tag,
                        "value originates from here",
                        value_tag.span,
                    ))
                }
            }
        }
    };

    Ok(stream.to_output_stream())
}
