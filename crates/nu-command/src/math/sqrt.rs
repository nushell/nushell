use crate::math::utils::run_with_elementwise;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct MathSqrt;

impl Command for MathSqrt {
    fn name(&self) -> &str {
        "math sqrt"
    }

    fn signature(&self) -> Signature {
        Signature::build("math sqrt")
            .input_output_types(vec![
                (Type::Number, Type::Float),
                (
                    Type::List(Box::new(Type::Number)),
                    Type::List(Box::new(Type::Float)),
                ),
                (Type::Range, Type::List(Box::new(Type::Number))),
                (Type::record(), Type::record()),
            ])
            .rest(
                "columns",
                SyntaxShape::CellPath,
                "The cell-paths/columns to operate on.",
            )
            .allow_variants_without_examples(true)
            .category(Category::Math)
    }

    fn description(&self) -> &str {
        "Returns the square root of the input number."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["square", "root"]
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
        let cell_paths: Vec<CellPath> = call.rest(engine_state, stack, 0)?;
        let head = call.head;
        run_with_elementwise(
            input,
            cell_paths,
            head,
            engine_state.signals(),
            true,
            move |value| operate(value, head),
        )
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let cell_paths: Vec<CellPath> = call.rest_const(working_set, 0)?;
        let head = call.head;
        run_with_elementwise(
            input,
            cell_paths,
            head,
            working_set.permanent().signals(),
            true,
            move |value| operate(value, head),
        )
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Compute the square root of each number in a list.",
                example: "[9 16] | math sqrt",
                result: Some(Value::list(
                    vec![Value::test_float(3.0), Value::test_float(4.0)],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Apply square root to list-valued columns in a record.",
                example: "{alice: [1 4 9], bob: [16 25 36]} | math sqrt",
                result: Some(Value::test_record(record! {
                    "alice" => Value::list(
                        vec![Value::test_float(1.0), Value::test_float(2.0), Value::test_float(3.0)],
                        Span::test_data(),
                    ),
                    "bob" => Value::list(
                        vec![Value::test_float(4.0), Value::test_float(5.0), Value::test_float(6.0)],
                        Span::test_data(),
                    ),
                })),
            },
            Example {
                description: "Apply square root to a single column using a cell path.",
                example: "{alice: [1 4 9], bob: [16 25 36]} | math sqrt alice",
                result: Some(Value::test_record(record! {
                    "alice" => Value::list(
                        vec![Value::test_float(1.0), Value::test_float(2.0), Value::test_float(3.0)],
                        Span::test_data(),
                    ),
                    "bob" => Value::list(
                        vec![Value::test_int(16), Value::test_int(25), Value::test_int(36)],
                        Span::test_data(),
                    ),
                })),
            },
        ]
    }
}

fn operate(value: Value, head: Span) -> Value {
    let span = value.span();
    match value {
        Value::Int { val, .. } => {
            let squared = (val as f64).sqrt();
            if squared.is_nan() {
                return error_negative_sqrt(head, span);
            }
            Value::float(squared, span)
        }
        Value::Float { val, .. } => {
            let squared = val.sqrt();
            if squared.is_nan() {
                return error_negative_sqrt(head, span);
            }
            Value::float(squared, span)
        }
        Value::Error { .. } => value,
        other => Value::error(
            ShellError::OnlySupportsThisInputType {
                exp_input_type: crate::math::utils::NUMBER_INPUT_TYPES.into(),
                wrong_type: other.get_type().to_string(),
                dst_span: head,
                src_span: other.span(),
            },
            head,
        ),
    }
}

fn error_negative_sqrt(head: Span, span: Span) -> Value {
    Value::error(
        ShellError::UnsupportedInput {
            msg: String::from("Can't square root a negative number"),
            input: "value originates from here".into(),
            msg_span: head,
            input_span: span,
        },
        span,
    )
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(MathSqrt)
    }
}
