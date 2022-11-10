use crate::input_handler::{operate, CellPathOnlyArgs};
use nu_engine::CallExt;
use nu_protocol::{
    ast::{Call, CellPath},
    engine::{Command, EngineState, Stack},
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, SyntaxShape,
    Type, Value,
};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "into binary"
    }

    fn signature(&self) -> Signature {
        Signature::build("into binary")
            .input_output_types(vec![
                (Type::Binary, Type::Binary),
                (Type::Int, Type::Binary),
                (Type::Number, Type::Binary),
                (Type::String, Type::Binary),
                (Type::Bool, Type::Binary),
                (Type::Filesize, Type::Binary),
                (Type::Date, Type::Binary),
            ])
            .allow_variants_without_examples(true) // TODO: supply exhaustive examples
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "for a data structure input, convert data at the given cell paths",
            )
            .category(Category::Conversions)
    }

    fn usage(&self) -> &str {
        "Convert value to a binary primitive"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["convert", "bytes"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        into_binary(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "convert string to a nushell binary primitive",
                example: "'This is a string that is exactly 52 characters long.' | into binary",
                result: Some(Value::Binary {
                    val: "This is a string that is exactly 52 characters long."
                        .to_string()
                        .as_bytes()
                        .to_vec(),
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "convert a number to a nushell binary primitive",
                example: "1 | into binary",
                result: Some(Value::Binary {
                    val: i64::from(1).to_le_bytes().to_vec(),
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "convert a boolean to a nushell binary primitive",
                example: "true | into binary",
                result: Some(Value::Binary {
                    val: i64::from(1).to_le_bytes().to_vec(),
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "convert a filesize to a nushell binary primitive",
                example: "ls | where name == LICENSE | get size | into binary",
                result: None,
            },
            Example {
                description: "convert a filepath to a nushell binary primitive",
                example: "ls | where name == LICENSE | get name | path expand | into binary",
                result: None,
            },
            Example {
                description: "convert a decimal to a nushell binary primitive",
                example: "1.234 | into binary",
                result: Some(Value::Binary {
                    val: 1.234f64.to_le_bytes().to_vec(),
                    span: Span::test_data(),
                }),
            },
        ]
    }
}

fn into_binary(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
    let head = call.head;
    let cell_paths: Vec<CellPath> = call.rest(engine_state, stack, 0)?;

    match input {
        PipelineData::ExternalStream { stdout: None, .. } => Ok(Value::Binary {
            val: vec![],
            span: head,
        }
        .into_pipeline_data()),
        PipelineData::ExternalStream {
            stdout: Some(stream),
            ..
        } => {
            // TODO: in the future, we may want this to stream out, converting each to bytes
            let output = stream.into_bytes()?;
            Ok(Value::Binary {
                val: output.item,
                span: head,
            }
            .into_pipeline_data())
        }
        _ => {
            let arg = CellPathOnlyArgs::from(cell_paths);
            operate(action, arg, input, call.head, engine_state.ctrlc.clone())
        }
    }
}

fn int_to_endian(n: i64) -> Vec<u8> {
    if cfg!(target_endian = "little") {
        n.to_le_bytes().to_vec()
    } else {
        n.to_be_bytes().to_vec()
    }
}

fn float_to_endian(n: f64) -> Vec<u8> {
    if cfg!(target_endian = "little") {
        n.to_le_bytes().to_vec()
    } else {
        n.to_be_bytes().to_vec()
    }
}

pub fn action(input: &Value, _args: &CellPathOnlyArgs, span: Span) -> Value {
    match input {
        Value::Binary { .. } => input.clone(),
        Value::Int { val, .. } => Value::Binary {
            val: int_to_endian(*val),
            span,
        },
        Value::Float { val, .. } => Value::Binary {
            val: float_to_endian(*val),
            span,
        },
        Value::Filesize { val, .. } => Value::Binary {
            val: int_to_endian(*val),
            span,
        },
        Value::String { val, .. } => Value::Binary {
            val: val.as_bytes().to_vec(),
            span,
        },
        Value::Bool { val, .. } => Value::Binary {
            val: int_to_endian(i64::from(*val)),
            span,
        },
        Value::Date { val, .. } => Value::Binary {
            val: val.format("%c").to_string().as_bytes().to_vec(),
            span,
        },

        _ => Value::Error {
            error: ShellError::UnsupportedInput("'into binary' for unsupported type".into(), span),
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
