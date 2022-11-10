use crate::input_handler::{operate, CellPathOnlyArgs};
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
            ])
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "for a data structure input, convert data at the given cell paths",
            )
            .category(Category::Conversions)
    }

    fn usage(&self) -> &str {
        "Convert value to filesize"
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
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let cell_paths: Vec<CellPath> = call.rest(engine_state, stack, 0)?;
        let args = CellPathOnlyArgs::from(cell_paths);
        operate(action, args, input, call.head, engine_state.ctrlc.clone())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Convert string to filesize in table",
                example: "[[bytes]; ['5'] [3.2] [4] [2kb]] | into filesize bytes",
                result: None,
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
                Err(error) => Value::Error { error },
            },
            Value::Nothing { .. } => Value::Filesize {
                val: 0,
                span: value_span,
            },
            _ => Value::Error {
                error: ShellError::UnsupportedInput(
                    "'into filesize' for unsupported type".into(),
                    value_span,
                ),
            },
        }
    } else {
        Value::Error {
            error: ShellError::UnsupportedInput(
                "'into filesize' for unsupported type".into(),
                span,
            ),
        }
    }
}
fn int_from_string(a_string: &str, span: Span) -> Result<i64, ShellError> {
    match a_string.trim().parse::<bytesize::ByteSize>() {
        Ok(n) => Ok(n.0 as i64),
        Err(_) => Err(ShellError::CantConvert(
            "int".into(),
            "string".into(),
            span,
            None,
        )),
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
