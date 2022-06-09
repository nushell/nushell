use nu_engine::CallExt;
use nu_protocol::{
    ast::{Call, CellPath},
    engine::{Command, EngineState, Stack},
    Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "into decimal"
    }

    fn signature(&self) -> Signature {
        Signature::build("into decimal").rest(
            "rest",
            SyntaxShape::CellPath,
            "optionally convert text into decimal by column paths",
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
        operate(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Convert string to integer in table",
                example: "[[num]; ['5.01']] | into decimal num",
                result: Some(Value::List(vec![Value::Record {
                    cols: vec!["num".to_string()],
                    vals: vec![Value::Float(5.01)],
                }])),
            },
            Example {
                description: "Convert string to integer",
                example: "'1.345' | into decimal",
                result: Some(Value::Float(1.345)),
            },
            Example {
                description: "Convert decimal to integer",
                example: "'-5.9' | into decimal",
                result: Some(Value::Float(-5.9)),
            },
        ]
    }
}

fn operate(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
    let head = call.head;
    let column_paths: Vec<CellPath> = call.rest(engine_state, stack, 0)?;

    input.map(
        move |v| {
            if column_paths.is_empty() {
                action(&v, head)
            } else {
                let mut ret = v;
                for path in &column_paths {
                    let r =
                        ret.update_cell_path(&path.members, Box::new(move |old| action(old, head)));
                    if let Err(error) = r {
                        return Value::Error(error);
                    }
                }

                ret
            }
        },
        engine_state.ctrlc.clone(),
        head,
    )
}

fn action(input: &Value, head: Span) -> Value {
    match input {
        Value::String(s) => {
            let other = s.trim();

            match other.parse::<f64>() {
                Ok(x) => Value::Float(x),
                Err(reason) => Value::Error(ShellError::CantConvert(
                    "float".to_string(),
                    reason.to_string(),
                    head,
                    None,
                )),
            }
        }
        Value::Int(v) => Value::Float(*v as f64),
        other => {
            let span = other.span();
            match span {
                Ok(s) => {
                    let got = format!("Expected a string, got {} instead", other.get_type());
                    Value::Error(ShellError::UnsupportedInput(got, s))
                }
                Err(e) => Value::Error(e),
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
        let word = Value::String("3.1415".into());
        let expected = Value::Float(3.1415);

        let actual = action(&word, Span::test_data());
        assert_eq!(actual, expected);
    }

    #[test]
    fn communicates_parsing_error_given_an_invalid_decimallike_string() {
        let decimal_str = Value::String("11.6anra".into());

        let actual = action(&decimal_str, Span::test_data());

        assert_eq!(actual.get_type(), Error);
    }

    #[test]
    fn int_to_decimal() {
        let decimal_str = Value::Int(10);
        let expected = Value::Float(10.0);
        let actual = action(&decimal_str, Span::test_data());

        assert_eq!(actual, expected);
    }
}
