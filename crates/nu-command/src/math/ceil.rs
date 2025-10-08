use crate::math::utils::ensure_bounded;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct MathCeil;

impl Command for MathCeil {
    fn name(&self) -> &str {
        "math ceil"
    }

    fn signature(&self) -> Signature {
        Signature::build("math ceil")
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
        "Returns the ceil of a number (smallest integer greater than or equal to that number)."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["ceiling", "round up", "rounding", "integer"]
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
        // This doesn't match explicit nulls
        if let PipelineData::Empty = input {
            return Err(ShellError::PipelineEmpty { dst_span: head });
        }
        if let PipelineData::Value(ref v @ Value::Range { ref val, .. }, ..) = input {
            let span = v.span();
            ensure_bounded(val, span, head)?;
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
            description: "Apply the ceil function to a list of numbers",
            example: "[1.5 2.3 -3.1] | math ceil",
            result: Some(Value::list(
                vec![Value::test_int(2), Value::test_int(3), Value::test_int(-3)],
                Span::test_data(),
            )),
        }]
    }
}

fn operate(value: Value, head: Span) -> Value {
    let span = value.span();
    match value {
        Value::Int { .. } => value,
        Value::Float { val, .. } => Value::int(val.ceil() as i64, span),
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

        test_examples(MathCeil {})
    }
}
