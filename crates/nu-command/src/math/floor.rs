use crate::math::utils::run_with_elementwise;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct MathFloor;

impl Command for MathFloor {
    fn name(&self) -> &str {
        "math floor"
    }

    fn signature(&self) -> Signature {
        Signature::build("math floor")
            .input_output_types(vec![
                (Type::Number, Type::Int),
                (Type::Duration, Type::Duration),
                (Type::Filesize, Type::Filesize),
                (
                    Type::List(Box::new(Type::Number)),
                    Type::List(Box::new(Type::Int)),
                ),
                (
                    Type::List(Box::new(Type::Duration)),
                    Type::List(Box::new(Type::Duration)),
                ),
                (
                    Type::List(Box::new(Type::Filesize)),
                    Type::List(Box::new(Type::Filesize)),
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
        "Returns the floor of a number (largest integer less than or equal to that number)."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["round down", "rounding", "integer"]
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
                description: "Apply the floor function to a list of numbers.",
                example: "[1.5 2.3 -3.1] | math floor",
                result: Some(Value::list(
                    vec![Value::test_int(1), Value::test_int(2), Value::test_int(-4)],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Apply the floor function to list-valued columns in a record.",
                example: "{alice: [1.2 2.7 3.5], bob: [4.1 5.9]} | math floor",
                result: Some(Value::test_record(record! {
                    "alice" => Value::list(
                        vec![Value::test_int(1), Value::test_int(2), Value::test_int(3)],
                        Span::test_data(),
                    ),
                    "bob" => Value::list(
                        vec![Value::test_int(4), Value::test_int(5)],
                        Span::test_data(),
                    ),
                })),
            },
            Example {
                description: "Apply the floor function to a single column using a cell path.",
                example: "{alice: [1.2 2.7 3.5], bob: [4.1 5.9]} | math floor alice",
                result: Some(Value::test_record(record! {
                    "alice" => Value::list(
                        vec![Value::test_int(1), Value::test_int(2), Value::test_int(3)],
                        Span::test_data(),
                    ),
                    "bob" => Value::list(
                        vec![Value::test_float(4.1), Value::test_float(5.9)],
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
        // Duration and filesize are already integer units (ns / bytes).
        Value::Int { .. } | Value::Duration { .. } | Value::Filesize { .. } => value,
        Value::Float { val, .. } => Value::int(val.floor() as i64, span),
        Value::Error { .. } => value,
        other => Value::error(
            ShellError::OnlySupportsThisInputType {
                exp_input_type: crate::math::utils::NUMERIC_INPUT_TYPES.into(),
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
        nu_test_support::test().examples(MathFloor)
    }
}
