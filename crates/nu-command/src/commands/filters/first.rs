use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, UntaggedValue};
use nu_source::Tagged;

pub struct First;

impl WholeStreamCommand for First {
    fn name(&self) -> &str {
        "first"
    }

    fn signature(&self) -> Signature {
        Signature::build("first").optional(
            "rows",
            SyntaxShape::Int,
            "starting from the front, the number of rows to return",
        )
    }

    fn usage(&self) -> &str {
        "Show only the first number of rows."
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        first(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Return the first item of a list/table",
                example: "echo [1 2 3] | first",
                result: Some(vec![UntaggedValue::int(1).into()]),
            },
            Example {
                description: "Return the first 2 items of a list/table",
                example: "echo [1 2 3] | first 2",
                result: Some(vec![
                    UntaggedValue::int(1).into(),
                    UntaggedValue::int(2).into(),
                ]),
            },
        ]
    }
}

fn first(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let rows: Option<Tagged<usize>> = args.opt(0)?;
    let tag = args.call_info.name_tag;

    let mut rows_desired = if let Some(quantity) = rows {
        *quantity
    } else {
        1
    };

    let mut input_peek = args.input.peekable();
    match &mut input_peek.next_if(|val| val.is_binary()) {
        Some(v) => match &v.value {
            // We already know it's a binary so we don't have to match
            // on the type of primitive
            UntaggedValue::Primitive(_) => {
                let bytes = match v.as_binary_vec() {
                    Ok(b) => b,
                    _ => {
                        return Err(ShellError::labeled_error(
                            "error converting data as_binary_vec",
                            "error conversion",
                            tag,
                        ))
                    }
                };
                // if the current 8192 chunk fits inside our rows_desired
                // carve it up and return it
                if bytes.len() >= rows_desired {
                    // We only want to see a certain amount of the binary
                    // so let's grab those parts
                    let output_bytes = bytes[0..rows_desired].to_vec();
                    Ok(OutputStream::one(UntaggedValue::binary(output_bytes)))
                } else {
                    // if we want more rows that the current chunk size (8192)
                    // we must gradually get bigger chunks while testing
                    // if it's within the requested rows_desired size
                    let mut bigger: Vec<u8> = vec![];
                    bigger.extend(bytes);
                    while bigger.len() < rows_desired {
                        match input_peek.next() {
                            Some(data) => match data.value.into_value(&tag).as_binary_vec() {
                                Ok(bits) => bigger.extend(bits),
                                _ => {
                                    return Err(ShellError::labeled_error(
                                        "error converting data as_binary_vec",
                                        "error conversion",
                                        tag,
                                    ))
                                }
                            },
                            _ => {
                                // We're at the end of our data so let's break out of this loop
                                // and set the rows_desired to the size of our data
                                rows_desired = bigger.len();
                                break;
                            }
                        }
                    }
                    let output_bytes = bigger[0..rows_desired].to_vec();
                    Ok(OutputStream::one(UntaggedValue::binary(output_bytes)))
                }
            }
            UntaggedValue::Row(_) => Ok(input_peek.take(rows_desired).into_output_stream()),
            UntaggedValue::Table(_) => Err(ShellError::labeled_error(
                "unsure how to handle UntaggedValue::Table",
                "found table",
                tag,
            )),
            UntaggedValue::Error(_) => Err(ShellError::labeled_error(
                "unsure how to handle UntaggedValue::Error",
                "found error",
                tag,
            )),
            UntaggedValue::Block(_) => Err(ShellError::labeled_error(
                "unsure how to handled UntaggedValue::Block",
                "found block",
                tag,
            )),
            #[cfg(all(not(target_arch = "wasm32"), feature = "dataframe"))]
            UntaggedValue::DataFrame(_) | UntaggedValue::FrameStruct(_) => {
                Err(ShellError::labeled_error(
                    "unsure how to handled dataframe struct",
                    "found dataframe",
                    tag,
                ))
            }
        },
        None => Ok(input_peek.take(rows_desired).into_output_stream()),
    }
}

#[cfg(test)]
mod tests {
    use super::First;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(First {})
    }
}
