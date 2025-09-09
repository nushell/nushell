use crate::math::utils::ensure_bounded;
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
            ])
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
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        if let PipelineData::Value(
            Value::Range {
                ref val,
                internal_span,
            },
            ..,
        ) = input
        {
            ensure_bounded(val.as_ref(), internal_span, head)?;
        }
        input.map(move |value| abs_helper(value, head), engine_state.signals())
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        if let PipelineData::Value(
            Value::Range {
                ref val,
                internal_span,
            },
            ..,
        ) = input
        {
            ensure_bounded(val.as_ref(), internal_span, head)?;
        }
        input.map(
            move |value| abs_helper(value, head),
            working_set.permanent().signals(),
        )
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Compute absolute value of each number in a list of numbers",
            example: "[-50 -100.0 25] | math abs",
            result: Some(Value::list(
                vec![
                    Value::test_int(50),
                    Value::test_float(100.0),
                    Value::test_int(25),
                ],
                Span::test_data(),
            )),
        }]
    }
}

fn abs_helper(val: Value, head: Span) -> Value {
    let span = val.span();
    match val {
        Value::Int { val, .. } => Value::int(val.abs(), span),
        Value::Float { val, .. } => Value::float(val.abs(), span),
        Value::Duration { val, .. } => Value::duration(val.abs(), span),
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
    fn test_examples() {
        use crate::test_examples;

        test_examples(MathAbs {})
    }
}
