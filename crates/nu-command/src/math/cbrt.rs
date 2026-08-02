use crate::math::utils::run_with_elementwise;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct MathCbrt;

impl Command for MathCbrt {
    fn name(&self) -> &str {
        "math cbrt"
    }

    fn signature(&self) -> Signature {
        Signature::build("math cbrt")
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
        "Returns the real-valued cube root of the input number."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["cube", "root"]
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
                description: "Compute the cube root of each number in a list.",
                example: "[8 -27] | math cbrt",
                result: Some(Value::list(
                    vec![Value::test_float(2.0), Value::test_float(-3.0)],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Compute the cube root of list-valued columns in a record.",
                example: "{alice: [8 27 64], bob: [125 216]} | math cbrt",
                result: Some(Value::test_record(record! {
                    "alice" => Value::list(
                        vec![Value::test_float(2.0), Value::test_float(3.0), Value::test_float(4.0)],
                        Span::test_data(),
                    ),
                    "bob" => Value::list(
                        vec![Value::test_float(5.0), Value::test_float(6.0)],
                        Span::test_data(),
                    ),
                })),
            },
            Example {
                description: "Compute the cube root of a single column using a cell path.",
                example: "{alice: [8 27 64], bob: [125 216]} | math cbrt alice",
                result: Some(Value::test_record(record! {
                    "alice" => Value::list(
                        vec![Value::test_float(2.0), Value::test_float(3.0), Value::test_float(4.0)],
                        Span::test_data(),
                    ),
                    "bob" => Value::list(
                        vec![Value::test_int(125), Value::test_int(216)],
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
        Value::Int { val, .. } => Value::float((val as f64).cbrt(), span),
        Value::Float { val, .. } => Value::float(val.cbrt(), span),
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(MathCbrt)
    }
}
