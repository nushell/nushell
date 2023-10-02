use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct BitsOr;

impl Command for BitsOr {
    fn name(&self) -> &str {
        "bits or"
    }

    fn signature(&self) -> Signature {
        Signature::build("bits or")
            .input_output_types(vec![
                (Type::Int, Type::Int),
                (
                    Type::List(Box::new(Type::Int)),
                    Type::List(Box::new(Type::Int)),
                ),
            ])
            .required("target", SyntaxShape::Int, "target int to perform bit or")
            .category(Category::Bits)
    }

    fn usage(&self) -> &str {
        "Performs bitwise or for ints."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["logic or"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let target: i64 = call.req(engine_state, stack, 0)?;

        // This doesn't match explicit nulls
        if matches!(input, PipelineData::Empty) {
            return Err(ShellError::PipelineEmpty { dst_span: head });
        }
        input.map(
            move |value| operate(value, target, head),
            engine_state.ctrlc.clone(),
        )
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Apply bits or to two numbers",
                example: "2 | bits or 6",
                result: Some(Value::test_int(6)),
            },
            Example {
                description: "Apply logical or to a list of numbers",
                example: "[8 3 2] | bits or 2",
                result: Some(Value::list(
                    vec![Value::test_int(10), Value::test_int(3), Value::test_int(2)],
                    Span::test_data(),
                )),
            },
        ]
    }
}

fn operate(value: Value, target: i64, head: Span) -> Value {
    let span = value.span();
    match value {
        Value::Int { val, .. } => Value::int(val | target, span),
        // Propagate errors by explicitly matching them before the final case.
        Value::Error { .. } => value,
        other => Value::error(
            ShellError::OnlySupportsThisInputType {
                exp_input_type: "int".into(),
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

        test_examples(BitsOr {})
    }
}
