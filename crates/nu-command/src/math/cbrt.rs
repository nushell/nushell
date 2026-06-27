use crate::math::utils::ensure_bounded;
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
            ])
            .allow_variants_without_examples(true)
            .category(Category::Math)
    }

    fn description(&self) -> &str {
        "Returns the real-valued cube root of the input number."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["cbrt", "cube", "root"]
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
            description: "Compute the cube root of each number in a list.",
            example: "[8 -27] | math cbrt",
            result: Some(Value::list(
                vec![Value::test_float(2.0), Value::test_float(-3.0)],
                Span::test_data(),
            )),
        }]
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
        nu_test_support::test().examples(MathCbrt)
    }
}
