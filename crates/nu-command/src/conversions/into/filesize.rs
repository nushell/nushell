use nu_cmd_base::input_handler::{operate, CellPathOnlyArgs};
use nu_engine::CallExt;
use nu_protocol::{
    ast::{Call, CellPath},
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "into filesize"
    }

    fn signature(&self) -> Signature {
        Signature::build("into filesize")
            .input_output_types(vec![
                (Type::Int, Type::Filesize),
                (Type::Number, Type::Filesize),
                (Type::String, Type::Filesize),
                (Type::Filesize, Type::Filesize),
                (Type::Table(vec![]), Type::Table(vec![])),
            ])
            .vectorizes_over_list(true)
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "for a data structure input, convert data at the given cell paths",
            )
            .category(Category::Conversions)
    }

    fn usage(&self) -> &str {
        "Convert value to filesize."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["convert", "number", "bytes"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let cell_paths: Vec<CellPath> = call.rest(engine_state, stack, 0)?;
        let args = CellPathOnlyArgs::from(cell_paths);
        operate(action, args, input, call.head, engine_state.ctrlc.clone())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Convert string to filesize in table",
                example: r#"[[device size]; ["/dev/sda1" "200"] ["/dev/loop0" "50"]] | into filesize size"#,
                result: Some(Value::List {
                    vals: vec![
                        Value::Record {
                            cols: vec!["device".to_string(), "size".to_string()],
                            vals: vec![
                                Value::String {
                                    val: "/dev/sda1".to_string(),
                                    span: Span::unknown(),
                                },
                                Value::Filesize {
                                    val: 200,
                                    span: Span::unknown(),
                                },
                            ],
                            span: Span::unknown(),
                        },
                        Value::Record {
                            cols: vec!["device".to_string(), "size".to_string()],
                            vals: vec![
                                Value::String {
                                    val: "/dev/sda2".to_string(),
                                    span: Span::unknown(),
                                },
                                Value::Filesize {
                                    val: 50,
                                    span: Span::unknown(),
                                },
                            ],
                            span: Span::unknown(),
                        },
                    ],
                    span: Span::unknown(),
                }),
            },
            Example {
                description: "Convert string to filesize",
                example: "'2' | into filesize",
                result: Some(Value::Filesize {
                    val: 2,
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Convert decimal to filesize",
                example: "8.3 | into filesize",
                result: Some(Value::Filesize {
                    val: 8,
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Convert int to filesize",
                example: "5 | into filesize",
                result: Some(Value::Filesize {
                    val: 5,
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Convert file size to filesize",
                example: "4KB | into filesize",
                result: Some(Value::Filesize {
                    val: 4000,
                    span: Span::test_data(),
                }),
            },
        ]
    }
}

pub fn action(input: &Value, _args: &CellPathOnlyArgs, span: Span) -> Value {
    if let Ok(value_span) = input.span() {
        match input {
            Value::Filesize { .. } => input.clone(),
            Value::Int { val, .. } => Value::Filesize {
                val: *val,
                span: value_span,
            },
            Value::Float { val, .. } => Value::Filesize {
                val: *val as i64,
                span: value_span,
            },
            Value::String { val, .. } => match int_from_string(val, value_span) {
                Ok(val) => Value::Filesize {
                    val,
                    span: value_span,
                },
                Err(error) => Value::Error {
                    error: Box::new(error),
                },
            },
            Value::Nothing { .. } => Value::Filesize {
                val: 0,
                span: value_span,
            },
            other => Value::Error {
                error: Box::new(ShellError::OnlySupportsThisInputType {
                    exp_input_type: "string and integer".into(),
                    wrong_type: other.get_type().to_string(),
                    dst_span: span,
                    src_span: value_span,
                }),
            },
        }
    } else {
        // Propagate existing errors
        input.clone()
    }
}
fn int_from_string(a_string: &str, span: Span) -> Result<i64, ShellError> {
    match a_string.trim().parse::<bytesize::ByteSize>() {
        Ok(n) => Ok(n.0 as i64),
        Err(_) => Err(ShellError::CantConvert {
            to_type: "int".into(),
            from_type: "string".into(),
            span,
            help: None,
        }),
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
