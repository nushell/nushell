use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Span, SpannedValue, Type,
};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "math sinh"
    }

    fn signature(&self) -> Signature {
        Signature::build("math sinh")
            .input_output_types(vec![
                (Type::Number, Type::Float),
                (
                    Type::List(Box::new(Type::Number)),
                    Type::List(Box::new(Type::Float)),
                ),
            ])
            .allow_variants_without_examples(true)
            .category(Category::Math)
    }

    fn usage(&self) -> &str {
        "Returns the hyperbolic sine of the number."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["trigonometry", "hyperbolic"]
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
        if matches!(input, PipelineData::Empty) {
            return Err(ShellError::PipelineEmpty { dst_span: head });
        }
        input.map(
            move |value| operate(value, head),
            engine_state.ctrlc.clone(),
        )
    }

    fn examples(&self) -> Vec<Example> {
        let e = std::f64::consts::E;
        vec![Example {
            description: "Apply the hyperbolic sine to 1",
            example: "1 | math sinh",
            result: Some(SpannedValue::test_float((e * e - 1.0) / (2.0 * e))),
        }]
    }
}

fn operate(value: SpannedValue, head: Span) -> SpannedValue {
    match value {
        numeric @ (SpannedValue::Int { .. } | SpannedValue::Float { .. }) => {
            let (val, span) = match numeric {
                SpannedValue::Int { val, span } => (val as f64, span),
                SpannedValue::Float { val, span } => (val, span),
                _ => unreachable!(),
            };

            SpannedValue::Float {
                val: val.sinh(),
                span,
            }
        }
        SpannedValue::Error { .. } => value,
        other => SpannedValue::Error {
            error: Box::new(ShellError::OnlySupportsThisInputType {
                exp_input_type: "numeric".into(),
                wrong_type: other.get_type().to_string(),
                dst_span: head,
                src_span: other.span(),
            }),
            span: head,
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
