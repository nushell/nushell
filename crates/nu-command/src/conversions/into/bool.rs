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
        "into bool"
    }

    fn signature(&self) -> Signature {
        Signature::build("into bool")
            .input_output_types(vec![
                (Type::Int, Type::Bool),
                (Type::Number, Type::Bool),
                (Type::String, Type::Bool),
                (Type::Bool, Type::Bool),
                (Type::List(Box::new(Type::Any)), Type::Table(vec![])),
            ])
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "for a data structure input, convert data at the given cell paths",
            )
            .category(Category::Conversions)
    }

    fn usage(&self) -> &str {
        "Convert value to boolean"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["convert", "boolean", "true", "false", "1", "0"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        into_bool(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        let span = Span::test_data();
        vec![
            Example {
                description: "Convert value to boolean in table",
                example: "echo [[value]; ['false'] ['1'] [0] [1.0] [true]] | into bool value",
                result: Some(Value::List {
                    vals: vec![
                        Value::Record {
                            cols: vec!["value".to_string()],
                            vals: vec![Value::boolean(false, span)],
                            span,
                        },
                        Value::Record {
                            cols: vec!["value".to_string()],
                            vals: vec![Value::boolean(true, span)],
                            span,
                        },
                        Value::Record {
                            cols: vec!["value".to_string()],
                            vals: vec![Value::boolean(false, span)],
                            span,
                        },
                        Value::Record {
                            cols: vec!["value".to_string()],
                            vals: vec![Value::boolean(true, span)],
                            span,
                        },
                        Value::Record {
                            cols: vec!["value".to_string()],
                            vals: vec![Value::boolean(true, span)],
                            span,
                        },
                    ],
                    span,
                }),
            },
            Example {
                description: "Convert bool to boolean",
                example: "true | into bool",
                result: Some(Value::boolean(true, span)),
            },
            Example {
                description: "convert integer to boolean",
                example: "1 | into bool",
                result: Some(Value::boolean(true, span)),
            },
            Example {
                description: "convert decimal to boolean",
                example: "0.3 | into bool",
                result: Some(Value::boolean(true, span)),
            },
            Example {
                description: "convert decimal string to boolean",
                example: "'0.0' | into bool",
                result: Some(Value::boolean(false, span)),
            },
            Example {
                description: "convert string to boolean",
                example: "'true' | into bool",
                result: Some(Value::boolean(true, span)),
            },
        ]
    }
}

fn into_bool(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let cell_paths: Vec<CellPath> = call.rest(engine_state, stack, 0)?;
    let args = CellPathOnlyArgs::from(cell_paths);
    operate(action, args, input, call.head, engine_state.ctrlc.clone())
}

fn string_to_boolean(s: &str, span: Span) -> Result<bool, ShellError> {
    match s.trim().to_lowercase().as_str() {
        "true" => Ok(true),
        "false" => Ok(false),
        o => {
            let val = o.parse::<f64>();
            match val {
                Ok(f) => Ok(f.abs() >= f64::EPSILON),
                Err(_) => Err(ShellError::CantConvert(
                    "boolean".to_string(),
                    "string".to_string(),
                    span,
                    Some(
                        r#"the strings "true" and "false" can be converted into a bool"#
                            .to_string(),
                    ),
                )),
            }
        }
    }
}

fn action(input: &Value, _args: &CellPathOnlyArgs, span: Span) -> Value {
    match input {
        Value::Bool { .. } => input.clone(),
        Value::Int { val, .. } => Value::Bool {
            val: *val != 0,
            span,
        },
        Value::Float { val, .. } => Value::Bool {
            val: val.abs() >= f64::EPSILON,
            span,
        },
        Value::String { val, .. } => match string_to_boolean(val, span) {
            Ok(val) => Value::Bool { val, span },
            Err(error) => Value::Error { error },
        },
        _ => Value::Error {
            error: ShellError::UnsupportedInput(
                "'into bool' does not support this input".into(),
                span,
            ),
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
