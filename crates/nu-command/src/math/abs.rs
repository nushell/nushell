use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Span, SpannedValue, Type,
};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
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
            ])
            .allow_variants_without_examples(true)
            .category(Category::Math)
    }

    fn usage(&self) -> &str {
        "Returns the absolute value of a number."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["absolute", "modulus", "positive", "distance"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        input.map(
            move |value| abs_helper(value, head),
            engine_state.ctrlc.clone(),
        )
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Compute absolute value of each number in a list of numbers",
            example: "[-50 -100.0 25] | math abs",
            result: Some(SpannedValue::List {
                vals: vec![
                    SpannedValue::test_int(50),
                    SpannedValue::test_float(100.0),
                    SpannedValue::test_int(25),
                ],
                span: Span::test_data(),
            }),
        }]
    }
}

fn abs_helper(val: SpannedValue, head: Span) -> SpannedValue {
    match val {
        SpannedValue::Int { val, span } => SpannedValue::int(val.abs(), span),
        SpannedValue::Float { val, span } => SpannedValue::Float {
            val: val.abs(),
            span,
        },
        SpannedValue::Duration { val, span } => SpannedValue::Duration {
            val: val.abs(),
            span,
        },
        SpannedValue::Error { .. } => val,
        other => SpannedValue::Error {
            error: Box::new(ShellError::OnlySupportsThisInputType {
                exp_input_type: "numeric".into(),
                wrong_type: other.get_type().to_string(),
                dst_span: head,
                src_span: other.expect_span(),
            }),
        },
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(SubCommand {})
    }
}
