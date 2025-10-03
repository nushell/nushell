use crate::math::utils::ensure_bounded;
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
                (
                    Type::List(Box::new(Type::Number)),
                    Type::List(Box::new(Type::Int)),
                ),
                (Type::Range, Type::List(Box::new(Type::Number))),
            ])
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
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        match input {
            // This doesn't match explicit nulls
            PipelineData::Empty => return Err(ShellError::PipelineEmpty { dst_span: head }),
            PipelineData::Value(ref value @ Value::Range { val: ref range, .. }, ..) => {
                ensure_bounded(range, value.span(), head)?
            }
            _ => (),
        }
        input.map(move |value| operate(value, head), engine_state.signals())
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
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
            move |value| operate(value, head),
            working_set.permanent().signals(),
        )
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Apply the floor function to a list of numbers",
            example: "[1.5 2.3 -3.1] | math floor",
            result: Some(Value::list(
                vec![Value::test_int(1), Value::test_int(2), Value::test_int(-4)],
                Span::test_data(),
            )),
        }]
    }
}

fn operate(value: Value, head: Span) -> Value {
    let span = value.span();
    match value {
        Value::Int { .. } => value,
        Value::Float { val, .. } => Value::int(val.floor() as i64, span),
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

        test_examples(MathFloor {})
    }
}
