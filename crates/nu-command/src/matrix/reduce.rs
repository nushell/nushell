use crate::matrix::MatrixValue;
use nu_engine::ClosureEval;
use nu_engine::command_prelude::*;
use nu_protocol::engine::Closure;

#[derive(Clone)]
pub struct MatrixReduce;

impl Command for MatrixReduce {
    fn name(&self) -> &str {
        "matrix reduce"
    }

    fn signature(&self) -> Signature {
        Signature::build("matrix reduce")
            .input_output_types(vec![(Type::Custom("matrix".into()), Type::Any)])
            .required(
                "closure",
                SyntaxShape::Closure(Some(vec![SyntaxShape::Any, SyntaxShape::Number])),
                "The closure to apply to the accumulation and each element.",
            )
            .named(
                "fold",
                SyntaxShape::Any,
                "The initial value for the accumulator",
                Some('f'),
            )
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Reduce all elements of a matrix to a single value."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["fold", "accumulate"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let closure: Closure = call.req(engine_state, stack, 0)?;
        let fold: Option<Value> = call.get_flag(engine_state, stack, "fold")?;
        let matrix = MatrixValue::from_value(&input.into_value(head)?)?;

        let mut iter = matrix.array.iter();
        let mut acc = if let Some(fold_val) = fold {
            fold_val
        } else {
            match iter.next() {
                Some(&first) => Value::float(first, head),
                None => {
                    return Err(ShellError::Generic(
                        nu_protocol::shell_error::generic::GenericError::new(
                            "Empty matrix",
                            "cannot reduce an empty matrix without --fold",
                            head,
                        ),
                    ));
                }
            }
        };

        let mut closure_eval = ClosureEval::new(engine_state, stack, closure);

        for &val in iter {
            engine_state.signals().check(&head)?;
            let element = Value::float(val, head);
            acc = closure_eval
                .add_arg(element)?
                .add_arg(acc.clone())?
                .run_with_input(PipelineData::value(acc, None))?
                .into_value(head)?;
        }

        Ok(acc.with_span(head).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example<'static>> {
        vec![
            Example {
                description: "Sum all elements of a 2x2 matrix",
                example: "[[1 2] [3 4]] | into matrix | matrix reduce --fold 0.0 {|acc e| $acc + $e}",
                result: Some(Value::test_float(10.0)),
            },
            Example {
                description: "Product of all elements in a matrix",
                example: "[[1 2] [3 4]] | into matrix | matrix reduce --fold 1.0 {|acc e| $acc * $e}",
                result: Some(Value::test_float(24.0)),
            },
            Example {
                description: "Sum without an initial value (uses first element as starting accumulator)",
                example: "[[1 2] [3 4]] | into matrix | matrix reduce {|acc e| $acc + $e}",
                result: Some(Value::test_float(10.0)),
            },
        ]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(MatrixReduce)
    }
}
