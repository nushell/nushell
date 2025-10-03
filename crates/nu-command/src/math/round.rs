use crate::math::utils::ensure_bounded;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct MathRound;

impl Command for MathRound {
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
                (Type::Range, Type::List(Box::new(Type::Number))),
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

    fn description(&self) -> &str {
        "Returns the input number rounded to the specified precision."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["approx", "closest", "nearest"]
    }

    fn is_const(&self) -> bool {
        true
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
        if let PipelineData::Empty = input {
            return Err(ShellError::PipelineEmpty { dst_span: head });
        }
        if let PipelineData::Value(ref v @ Value::Range { ref val, .. }, ..) = input {
            let span = v.span();
            ensure_bounded(val, span, head)?;
        }
        input.map(
            move |value| operate(value, head, precision_param),
            engine_state.signals(),
        )
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let precision_param: Option<i64> = call.get_flag_const(working_set, "precision")?;
        let head = call.head;
        // This doesn't match explicit nulls
        if let PipelineData::Empty = input {
            return Err(ShellError::PipelineEmpty { dst_span: head });
        }
        if let PipelineData::Value(ref v @ Value::Range { ref val, .. }, ..) = input {
            let span = v.span();
            ensure_bounded(val, span, head)?;
        }
        input.map(
            move |value| operate(value, head, precision_param),
            working_set.permanent().signals(),
        )
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Apply the round function to a list of numbers",
                example: "[1.5 2.3 -3.1] | math round",
                result: Some(Value::list(
                    vec![Value::test_int(2), Value::test_int(2), Value::test_int(-3)],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Apply the round function with precision specified",
                example: "[1.555 2.333 -3.111] | math round --precision 2",
                result: Some(Value::list(
                    vec![
                        Value::test_float(1.56),
                        Value::test_float(2.33),
                        Value::test_float(-3.11),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Apply negative precision to a list of numbers",
                example: "[123, 123.3, -123.4] | math round --precision -1",
                result: Some(Value::list(
                    vec![
                        Value::test_int(120),
                        Value::test_int(120),
                        Value::test_int(-120),
                    ],
                    Span::test_data(),
                )),
            },
        ]
    }
}

fn operate(value: Value, head: Span, precision: Option<i64>) -> Value {
    // We treat int values as float values in order to avoid code repetition in the match closure
    let span = value.span();
    let value = if let Value::Int { val, .. } = value {
        Value::float(val as f64, span)
    } else {
        value
    };

    match value {
        Value::Float { val, .. } => match precision {
            Some(precision_number) => Value::float(
                (val * ((10_f64).powf(precision_number as f64))).round()
                    / (10_f64).powf(precision_number as f64),
                span,
            ),
            None => Value::int(val.round() as i64, span),
        },
        Value::Error { .. } => value,
        other => Value::error(
            ShellError::OnlySupportsThisInputType {
                exp_input_type: "numeric".into(),
                wrong_type: other.get_type().to_string(),
                dst_span: head,
                src_span: other.span(),
            },
            head,
        ),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(MathRound {})
    }
}
