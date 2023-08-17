use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Span, SpannedValue, SyntaxShape, Type,
};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "math round"
    }

    fn signature(&self) -> Signature {
        Signature::build("math round")
            .input_output_types(vec![
                (Type::Number, Type::Number),
                (
                    Type::List(Box::new(Type::Number)),
                    Type::List(Box::new(Type::Number)),
                ),
            ])
            .allow_variants_without_examples(true)
            .named(
                "precision",
                SyntaxShape::Number,
                "digits of precision",
                Some('p'),
            )
            .category(Category::Math)
    }

    fn usage(&self) -> &str {
        "Returns the input number rounded to the specified precision."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["approx", "closest", "nearest"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let precision_param: Option<i64> = call.get_flag(engine_state, stack, "precision")?;
        let head = call.head;
        // This doesn't match explicit nulls
        if matches!(input, PipelineData::Empty) {
            return Err(ShellError::PipelineEmpty { dst_span: head });
        }
        input.map(
            move |value| operate(value, head, precision_param),
            engine_state.ctrlc.clone(),
        )
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Apply the round function to a list of numbers",
                example: "[1.5 2.3 -3.1] | math round",
                result: Some(SpannedValue::List {
                    vals: vec![
                        SpannedValue::test_int(2),
                        SpannedValue::test_int(2),
                        SpannedValue::test_int(-3),
                    ],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Apply the round function with precision specified",
                example: "[1.555 2.333 -3.111] | math round -p 2",
                result: Some(SpannedValue::List {
                    vals: vec![
                        SpannedValue::test_float(1.56),
                        SpannedValue::test_float(2.33),
                        SpannedValue::test_float(-3.11),
                    ],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Apply negative precision to a list of numbers",
                example: "[123, 123.3, -123.4] | math round -p -1",
                result: Some(SpannedValue::List {
                    vals: vec![
                        SpannedValue::test_int(120),
                        SpannedValue::test_int(120),
                        SpannedValue::test_int(-120),
                    ],
                    span: Span::test_data(),
                }),
            },
        ]
    }
}

fn operate(value: SpannedValue, head: Span, precision: Option<i64>) -> SpannedValue {
    // We treat int values as float values in order to avoid code repetition in the match closure
    let value = if let SpannedValue::Int { val, span } = value {
        SpannedValue::Float {
            val: val as f64,
            span,
        }
    } else {
        value
    };

    match value {
        SpannedValue::Float { val, span } => match precision {
            Some(precision_number) => SpannedValue::Float {
                val: ((val * ((10_f64).powf(precision_number as f64))).round()
                    / (10_f64).powf(precision_number as f64)),
                span,
            },
            None => SpannedValue::Int {
                val: val.round() as i64,
                span,
            },
        },
        SpannedValue::Error { .. } => value,
        other => SpannedValue::Error {
            error: Box::new(ShellError::OnlySupportsThisInputType {
                exp_input_type: "numeric".into(),
                wrong_type: other.get_type().to_string(),
                dst_span: head,
                src_span: other.span(),
            }),
            span: head,
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
