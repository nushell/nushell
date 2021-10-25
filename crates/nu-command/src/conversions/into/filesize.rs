use nu_protocol::{
    ast::Call,
    engine::{Command, EvaluationContext},
    Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "into filesize"
    }

    fn signature(&self) -> Signature {
        Signature::build("into filesize").rest(
            "rest",
            SyntaxShape::CellPath,
            "column paths to convert to filesize (for table input)",
        )
    }

    fn usage(&self) -> &str {
        "Convert value to filesize"
    }

    fn run(
        &self,
        context: &EvaluationContext,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        into_filesize(context, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            // Example {
            //     description: "Convert string to filesize in table",
            //     example: "[[bytes]; ['5'] [3.2] [4] [2kb]] | into filesize bytes",
            //     result: Some(Value::List {
            //         vals: vec![
            //             Value::Record {
            //                 cols: vec!["bytes".to_string()],
            //                 vals: vec![Value::Filesize {
            //                     val: 5,
            //                     span: Span::unknown(),
            //                 }],
            //                 span: Span::unknown(),
            //             },
            //             Value::Record {
            //                 cols: vec!["bytes".to_string()],
            //                 vals: vec![Value::Filesize {
            //                     val: 3,
            //                     span: Span::unknown(),
            //                 }],
            //                 span: Span::unknown(),
            //             },
            //             Value::Record {
            //                 cols: vec!["bytes".to_string()],
            //                 vals: vec![Value::Filesize {
            //                     val: 4,
            //                     span: Span::unknown(),
            //                 }],
            //                 span: Span::unknown(),
            //             },
            //             Value::Record {
            //                 cols: vec!["bytes".to_string()],
            //                 vals: vec![Value::Filesize {
            //                     val: 2000,
            //                     span: Span::unknown(),
            //                 }],
            //                 span: Span::unknown(),
            //             },
            //         ],
            //         span: Span::unknown(),
            //     }),
            // },
            Example {
                description: "Convert string to filesize",
                example: "'2' | into filesize",
                result: Some(Value::Filesize {
                    val: 2,
                    span: Span::unknown(),
                }),
            },
            Example {
                description: "Convert decimal to filesize",
                example: "8.3 | into filesize",
                result: Some(Value::Filesize {
                    val: 8,
                    span: Span::unknown(),
                }),
            },
            Example {
                description: "Convert int to filesize",
                example: "5 | into filesize",
                result: Some(Value::Filesize {
                    val: 5,
                    span: Span::unknown(),
                }),
            },
            Example {
                description: "Convert file size to filesize",
                example: "4KB | into filesize",
                result: Some(Value::Filesize {
                    val: 4000,
                    span: Span::unknown(),
                }),
            },
        ]
    }
}

fn into_filesize(
    _context: &EvaluationContext,
    call: &Call,
    input: PipelineData,
) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
    let head = call.head;
    // let call_paths: Vec<ColumnPath> = args.rest(0)?;

    Ok(input
        .map(move |v| {
            action(v, head)

            // FIXME: Add back cell_path support
            // if column_paths.is_empty() {
            //     action(&v, v.tag())
            // } else {
            //     let mut ret = v;
            //     for path in &column_paths {
            //         ret = ret.swap_data_by_column_path(
            //             path,
            //             Box::new(move |old| action(old, old.tag())),
            //         )?;
            //     }

            //     Ok(ret)
            // }
        })
        .into_pipeline_data())
}

pub fn action(input: Value, span: Span) -> Value {
    match input {
        Value::Filesize { .. } => input,
        Value::Int { val, .. } => Value::Filesize { val, span },
        Value::Float { val, .. } => Value::Filesize {
            val: val as i64,
            span,
        },
        Value::String { val, .. } => match int_from_string(&val, span) {
            Ok(val) => Value::Filesize { val, span },
            Err(error) => Value::Error { error },
        },
        _ => Value::Error {
            error: ShellError::UnsupportedInput(
                "'into filesize' for unsupported type".into(),
                span,
            ),
        },
    }
}
fn int_from_string(a_string: &str, span: Span) -> Result<i64, ShellError> {
    match a_string.parse::<bytesize::ByteSize>() {
        Ok(n) => Ok(n.0 as i64),
        Err(_) => Err(ShellError::CantConvert("int".into(), span)),
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
