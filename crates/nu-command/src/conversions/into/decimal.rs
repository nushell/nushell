use nu_cmd_base::input_handler::{operate, CellPathOnlyArgs};
use nu_engine::CallExt;
use nu_protocol::{
    ast::{Call, CellPath},
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, Record, ShellError, Signature, Span, SyntaxShape, Type, Value,
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
                (Type::Int, Type::Float),
                (Type::String, Type::Float),
                (Type::Bool, Type::Float),
                (Type::Float, Type::Float),
                (Type::Table(vec![]), Type::Table(vec![])),
                (Type::Record(vec![]), Type::Record(vec![])),
                (
                    Type::List(Box::new(Type::Any)),
                    Type::List(Box::new(Type::Float)),
                ),
            ])
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "for a data structure input, convert data at the given cell paths",
            )
            .allow_variants_without_examples(true)
            .category(Category::Conversions)
    }

    fn usage(&self) -> &str {
        "deprecated: convert data into a floating point number."
    }

    fn extra_usage(&self) -> &str {
        "Use `into float` instead"
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
    ) -> Result<PipelineData, ShellError> {
        nu_protocol::report_error_new(
            engine_state,
            &ShellError::GenericError(
                "Deprecated command".into(),
                "`into decimal` is deprecated and will be removed in 0.86.".into(),
                Some(call.head),
                Some("Use `into float` instead".into()),
                vec![],
            ),
        );
        let cell_paths: Vec<CellPath> = call.rest(engine_state, stack, 0)?;
        let args = CellPathOnlyArgs::from(cell_paths);
        operate(action, args, input, call.head, engine_state.ctrlc.clone())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Convert string to float in table",
                example: "[[num]; ['5.01']] | into decimal num",
                result: Some(Value::list(
                    vec![Value::test_record(Record {
                        cols: vec!["num".to_string()],
                        vals: vec![Value::test_float(5.01)],
                    })],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Convert string to float",
                example: "'1.345' | into decimal",
                result: Some(Value::test_float(1.345)),
            },
            Example {
                description: "Coerce list of ints and floats to float",
                example: "[4 -5.9] | into decimal",
                result: Some(Value::test_list(vec![
                    Value::test_float(4.0),
                    Value::test_float(-5.9),
                ])),
            },
            Example {
                description: "Convert boolean to float",
                example: "true | into decimal",
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
                exp_input_type: "string, integer or bool".into(),
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
