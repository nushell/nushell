use nu_cmd_base::input_handler::{operate, CellPathOnlyArgs};
use nu_engine::CallExt;
use nu_protocol::{
    ast::{Call, CellPath},
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, SpannedValue, SyntaxShape, Type,
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
        "Convert text into a decimal."
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
        let cell_paths: Vec<CellPath> = call.rest(engine_state, stack, 0)?;
        let args = CellPathOnlyArgs::from(cell_paths);
        operate(action, args, input, call.head, engine_state.ctrlc.clone())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Convert string to decimal in table",
                example: "[[num]; ['5.01']] | into decimal num",
                result: Some(SpannedValue::List {
                    vals: vec![SpannedValue::Record {
                        cols: vec!["num".to_string()],
                        vals: vec![SpannedValue::test_float(5.01)],
                        span: Span::test_data(),
                    }],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Convert string to decimal",
                example: "'1.345' | into decimal",
                result: Some(SpannedValue::test_float(1.345)),
            },
            Example {
                description: "Coerce list of ints and floats to float",
                example: "[4 -5.9] | into decimal",
                result: Some(SpannedValue::test_list(vec![
                    SpannedValue::test_float(4.0),
                    SpannedValue::test_float(-5.9),
                ])),
            },
            Example {
                description: "Convert boolean to decimal",
                example: "true | into decimal",
                result: Some(SpannedValue::test_float(1.0)),
            },
        ]
    }
}

fn action(input: &SpannedValue, _args: &CellPathOnlyArgs, head: Span) -> SpannedValue {
    match input {
        SpannedValue::Float { .. } => input.clone(),
        SpannedValue::String { val: s, span } => {
            let other = s.trim();

            match other.parse::<f64>() {
                Ok(x) => SpannedValue::float(x, head),
                Err(reason) => SpannedValue::Error {
                    error: Box::new(ShellError::CantConvert {
                        to_type: "float".to_string(),
                        from_type: reason.to_string(),
                        span: *span,
                        help: None,
                    }),
                    span: *span,
                },
            }
        }
        SpannedValue::Int { val: v, span } => SpannedValue::float(*v as f64, *span),
        SpannedValue::Bool { val: b, span } => SpannedValue::Float {
            val: match b {
                true => 1.0,
                false => 0.0,
            },
            span: *span,
        },
        // Propagate errors by explicitly matching them before the final case.
        SpannedValue::Error { .. } => input.clone(),
        other => SpannedValue::Error {
            error: Box::new(ShellError::OnlySupportsThisInputType {
                exp_input_type: "string, integer or bool".into(),
                wrong_type: other.get_type().to_string(),
                dst_span: head,
                src_span: other.span(),
            }),
            span: head,
        },
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
        let word = SpannedValue::test_string("3.1415");
        let expected = SpannedValue::test_float(3.1415);

        let actual = action(&word, &CellPathOnlyArgs::from(vec![]), Span::test_data());
        assert_eq!(actual, expected);
    }

    #[test]
    fn communicates_parsing_error_given_an_invalid_decimallike_string() {
        let decimal_str = SpannedValue::test_string("11.6anra");

        let actual = action(
            &decimal_str,
            &CellPathOnlyArgs::from(vec![]),
            Span::test_data(),
        );

        assert_eq!(actual.get_type(), Error);
    }

    #[test]
    fn int_to_decimal() {
        let decimal_str = SpannedValue::test_int(10);
        let expected = SpannedValue::test_float(10.0);
        let actual = action(
            &decimal_str,
            &CellPathOnlyArgs::from(vec![]),
            Span::test_data(),
        );

        assert_eq!(actual, expected);
    }
}
