use crate::input_handler::{operate, CellPathOnlyArgs};
use nu_engine::CallExt;
use nu_protocol::{
    ast::{Call, CellPath},
    engine::{Command, EngineState, Stack},
    Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "into decimal"
    }

    fn signature(&self) -> Signature {
        Signature::build("into decimal")
            .input_output_types(vec![
                (Type::String, Type::Number),
                (Type::Bool, Type::Number),
            ])
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "for a data structure input, convert data at the given cell paths",
            )
    }

    fn usage(&self) -> &str {
        "Convert text into a decimal"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["convert", "number", "floating"]
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
                description: "Convert string to decimal in table",
                example: "[[num]; ['5.01']] | into decimal num",
                result: Some(Value::List {
                    vals: vec![Value::Record {
                        cols: vec!["num".to_string()],
                        vals: vec![Value::test_float(5.01)],
                        span: Span::test_data(),
                    }],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Convert string to decimal",
                example: "'1.345' | into decimal",
                result: Some(Value::test_float(1.345)),
            },
            Example {
                description: "Convert decimal to decimal",
                example: "'-5.9' | into decimal",
                result: Some(Value::test_float(-5.9)),
            },
            Example {
                description: "Convert boolean to decimal",
                example: "true | into decimal",
                result: Some(Value::test_float(1.0)),
            },
        ]
    }
}

fn action(input: &Value, _args: &CellPathOnlyArgs, head: Span) -> Value {
    match input {
        Value::String { val: s, span } => {
            let other = s.trim();

            match other.parse::<f64>() {
                Ok(x) => Value::Float { val: x, span: head },
                Err(reason) => Value::Error {
                    error: ShellError::CantConvert(
                        "float".to_string(),
                        reason.to_string(),
                        *span,
                        None,
                    ),
                },
            }
        }
        Value::Int { val: v, span } => Value::Float {
            val: *v as f64,
            span: *span,
        },
        Value::Bool { val: b, span } => Value::Float {
            val: match b {
                true => 1.0,
                false => 0.0,
            },
            span: *span,
        },
        other => {
            let span = other.span();
            match span {
                Ok(s) => {
                    let got = format!("Expected a string, got {} instead", other.get_type());
                    Value::Error {
                        error: ShellError::UnsupportedInput(got, s),
                    }
                }
                Err(e) => Value::Error { error: e },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nu_protocol::Type::Error;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(SubCommand {})
    }

    #[test]
    #[allow(clippy::approx_constant)]
    fn string_to_decimal() {
        let word = Value::test_string("3.1415");
        let expected = Value::test_float(3.1415);

        let actual = action(&word, &CellPathOnlyArgs::from(vec![]), Span::test_data());
        assert_eq!(actual, expected);
    }

    #[test]
    fn communicates_parsing_error_given_an_invalid_decimallike_string() {
        let decimal_str = Value::test_string("11.6anra");

        let actual = action(
            &decimal_str,
            &CellPathOnlyArgs::from(vec![]),
            Span::test_data(),
        );

        assert_eq!(actual.get_type(), Error);
    }

    #[test]
    fn int_to_decimal() {
        let decimal_str = Value::test_int(10);
        let expected = Value::test_float(10.0);
        let actual = action(
            &decimal_str,
            &CellPathOnlyArgs::from(vec![]),
            Span::test_data(),
        );

        assert_eq!(actual, expected);
    }
}
