use crate::math::utils::run_with_elementwise;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct MathAbs;

impl Command for MathAbs {
    fn name(&self) -> &str {
        "math abs"
    }

    fn signature(&self) -> Signature {
        Signature::build("math abs")
            .input_output_types(vec![
                (Type::Number, Type::Number),
                (Type::Duration, Type::Duration),
                (
                    Type::List(Box::new(Type::Number)),
                    Type::List(Box::new(Type::Number)),
                ),
                (
                    Type::List(Box::new(Type::Duration)),
                    Type::List(Box::new(Type::Duration)),
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
        "Returns the absolute value of a number."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["absolute", "modulus", "positive", "distance"]
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
            false,
            move |value| abs_helper(value, head),
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
            false,
            move |value| abs_helper(value, head),
        )
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Compute absolute value of each number in a list of numbers.",
                example: "[-50 -100.0 25] | math abs",
                result: Some(Value::list(
                    vec![
                        Value::test_int(50),
                        Value::test_float(100.0),
                        Value::test_int(25),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Compute the absolute value of list-valued columns in a record.",
                example: "{alice: [-1 -2 -3], bob: [-4 -5]} | math abs",
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
                description: "Compute the absolute value of a single column using a cell path.",
                example: "{alice: [-1 -2 -3], bob: [-4 -5]} | math abs alice",
                result: Some(Value::test_record(record! {
                    "alice" => Value::list(
                        vec![Value::test_int(1), Value::test_int(2), Value::test_int(3)],
                        Span::test_data(),
                    ),
                    "bob" => Value::list(
                        vec![Value::test_int(-4), Value::test_int(-5)],
                        Span::test_data(),
                    ),
                })),
            },
        ]
    }
}

fn abs_helper(val: Value, head: Span) -> Value {
    let span = val.span();
    match val {
        Value::Int { val, .. } => match val.checked_abs() {
            Some(abs) => Value::int(abs, span),
            None => Value::error(
                ShellError::OperatorOverflow {
                    msg: "absolute value operation overflowed".into(),
                    span,
                    help: Some(format!(
                        "the absolute value of {val} cannot be represented as a 64-bit integer"
                    )),
                },
                span,
            ),
        },
        Value::Float { val, .. } => Value::float(val.abs(), span),
        Value::Duration { val, .. } => match val.checked_abs() {
            Some(abs) => Value::duration(abs, span),
            None => Value::error(
                ShellError::OperatorOverflow {
                    msg: "absolute value operation overflowed".into(),
                    span,
                    help: Some(
                        "the absolute value of the minimum duration cannot be represented".into(),
                    ),
                },
                span,
            ),
        },
        Value::Error { .. } => val,
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
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(MathAbs)
    }
}
