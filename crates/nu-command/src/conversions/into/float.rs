use nu_cmd_base::input_handler::{CellPathOnlyArgs, operate};
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct IntoFloat;

impl Command for IntoFloat {
    fn name(&self) -> &str {
        "into float"
    }

    fn signature(&self) -> Signature {
        Signature::build("into float")
            .input_output_types(vec![
                (Type::Int, Type::Float),
                (Type::String, Type::Float),
                (Type::Bool, Type::Float),
                (Type::Float, Type::Float),
                (Type::table(), Type::table()),
                (Type::record(), Type::record()),
                (
                    Type::List(Box::new(Type::Any)),
                    Type::List(Box::new(Type::Float)),
                ),
            ])
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "For a data structure input, convert data at the given cell paths.",
            )
            .allow_variants_without_examples(true)
            .category(Category::Conversions)
    }

    fn description(&self) -> &str {
        "Convert data into floating point number."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["convert", "number", "floating", "decimal"]
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
        operate(action, args, input, call.head, engine_state.signals())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Convert string to float in table",
                example: "[[num]; ['5.01']] | into float num",
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "num" => Value::test_float(5.01),
                })])),
            },
            Example {
                description: "Convert string to floating point number",
                example: "'1.345' | into float",
                result: Some(Value::test_float(1.345)),
            },
            Example {
                description: "Coerce list of ints and floats to float",
                example: "[4 -5.9] | into float",
                result: Some(Value::test_list(vec![
                    Value::test_float(4.0),
                    Value::test_float(-5.9),
                ])),
            },
            Example {
                description: "Convert boolean to float",
                example: "true | into float",
                result: Some(Value::test_float(1.0)),
            },
        ]
    }
}

fn action(input: &Value, _args: &CellPathOnlyArgs, head: Span) -> Value {
    let span = input.span();
    match input {
        Value::Float { .. } => input.clone(),
        Value::String { val: s, .. } => {
            let other = s.trim();

            match other.parse::<f64>() {
                Ok(x) => Value::float(x, head),
                Err(reason) => Value::error(
                    ShellError::CantConvert {
                        to_type: "float".to_string(),
                        from_type: reason.to_string(),
                        span,
                        help: None,
                    },
                    span,
                ),
            }
        }
        Value::Int { val: v, .. } => Value::float(*v as f64, span),
        Value::Bool { val: b, .. } => Value::float(
            match b {
                true => 1.0,
                false => 0.0,
            },
            span,
        ),
        // Propagate errors by explicitly matching them before the final case.
        Value::Error { .. } => input.clone(),
        other => Value::error(
            ShellError::OnlySupportsThisInputType {
                exp_input_type: "string, int or bool".into(),
                wrong_type: other.get_type().to_string(),
                dst_span: head,
                src_span: other.span(),
            },
            head,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nu_protocol::Type::Error;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(IntoFloat {})
    }

    #[test]
    #[allow(clippy::approx_constant)]
    fn string_to_float() {
        let word = Value::test_string("3.1415");
        let expected = Value::test_float(3.1415);

        let actual = action(&word, &CellPathOnlyArgs::from(vec![]), Span::test_data());
        assert_eq!(actual, expected);
    }

    #[test]
    fn communicates_parsing_error_given_an_invalid_floatlike_string() {
        let invalid_str = Value::test_string("11.6anra");

        let actual = action(
            &invalid_str,
            &CellPathOnlyArgs::from(vec![]),
            Span::test_data(),
        );

        assert_eq!(actual.get_type(), Error);
    }

    #[test]
    fn int_to_float() {
        let input_int = Value::test_int(10);
        let expected = Value::test_float(10.0);
        let actual = action(
            &input_int,
            &CellPathOnlyArgs::from(vec![]),
            Span::test_data(),
        );

        assert_eq!(actual, expected);
    }
}
