use nu_protocol::{
    ast::Call,
    engine::{Command, EvaluationContext},
    Example, IntoValueStream, ShellError, Signature, Span, SyntaxShape, Value,
};

pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "into int"
    }

    fn signature(&self) -> Signature {
        Signature::build("into int").rest(
            "rest",
            SyntaxShape::CellPath,
            "column paths to convert to int (for table input)",
        )
    }

    fn usage(&self) -> &str {
        "Convert value to integer"
    }

    fn run(
        &self,
        context: &EvaluationContext,
        call: &Call,
        input: Value,
    ) -> Result<nu_protocol::Value, nu_protocol::ShellError> {
        into_int(context, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            // Example {
            //     description: "Convert string to integer in table",
            //     example: "echo [[num]; ['-5'] [4] [1.5]] | into int num",
            //     result: Some(vec![
            //         UntaggedValue::row(indexmap! {
            //             "num".to_string() => UntaggedValue::int(-5).into(),
            //         })
            //         .into(),
            //         UntaggedValue::row(indexmap! {
            //             "num".to_string() => UntaggedValue::int(4).into(),
            //         })
            //         .into(),
            //         UntaggedValue::row(indexmap! {
            //             "num".to_string() => UntaggedValue::int(1).into(),
            //         })
            //         .into(),
            //     ]),
            // },
            Example {
                description: "Convert string to integer",
                example: "'2' | into int",
                result: Some(Value::test_int(2)),
            },
            Example {
                description: "Convert decimal to integer",
                example: "5.9 | into int",
                result: Some(Value::test_int(5)),
            },
            Example {
                description: "Convert decimal string to integer",
                example: "'5.9' | into int",
                result: Some(Value::test_int(5)),
            },
            Example {
                description: "Convert file size to integer",
                example: "4KB | into int",
                result: Some(Value::Int {
                    val: 4000,
                    span: Span::unknown(),
                }),
            },
            Example {
                description: "Convert bool to integer",
                example: "[$false, $true] | into int",
                result: Some(Value::Stream {
                    stream: vec![Value::test_int(0), Value::test_int(1)]
                        .into_iter()
                        .into_value_stream(),
                    span: Span::unknown(),
                }),
            },
        ]
    }
}

fn into_int(
    _context: &EvaluationContext,
    call: &Call,
    input: Value,
) -> Result<nu_protocol::Value, nu_protocol::ShellError> {
    let head = call.head;
    // let column_paths: Vec<CellPath> = call.rest(context, 0)?;

    input.map(head, move |v| {
        action(v, head)
        // FIXME: Add back cell_path support
        // if column_paths.is_empty() {
        //     action(&v, v.tag())
        // } else {
        //     let mut ret = v;
        //     for path in &column_paths {
        //         ret = ret
        //             .swap_data_by_column_path(path, Box::new(move |old| action(old, old.tag())))?;
        //     }

        //     Ok(ret)
        // }
    })
}

pub fn action(input: Value, span: Span) -> Value {
    match input {
        Value::Int { .. } => input,
        Value::Filesize { val, .. } => Value::Int { val, span },
        Value::Float { val, .. } => Value::Int {
            val: val as i64,
            span,
        },
        Value::String { val, .. } => match int_from_string(&val, span) {
            Ok(val) => Value::Int { val, span },
            Err(error) => Value::Error { error },
        },
        Value::Bool { val, .. } => {
            if val {
                Value::Int { val: 1, span }
            } else {
                Value::Int { val: 0, span }
            }
        }
        _ => Value::Error {
            error: ShellError::UnsupportedInput("'into int' for unsupported type".into(), span),
        },
    }
}

fn int_from_string(a_string: &str, span: Span) -> Result<i64, ShellError> {
    match a_string.parse::<i64>() {
        Ok(n) => Ok(n),
        Err(_) => match a_string.parse::<f64>() {
            Ok(f) => Ok(f as i64),
            _ => Err(ShellError::CantConvert("into int".into(), span)),
        },
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(SubCommand {})
    }
}
