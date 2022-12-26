use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "bits xor"
    }

    fn signature(&self) -> Signature {
        Signature::build("bits xor")
            .input_output_types(vec![(Type::Int, Type::Int)])
            .vectorizes_over_list(true)
            .required(
                "target",
                SyntaxShape::Int,
                "target integer to perform bit xor",
            )
            .category(Category::Bits)
    }

    fn usage(&self) -> &str {
        "Performs bitwise xor for integers"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["logic xor"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let head = call.head;
        let target: i64 = call.req(engine_state, stack, 0)?;
        // This doesn't match explicit nulls
        if matches!(input, PipelineData::Empty) {
            return Err(ShellError::PipelineEmpty(head));
        }
        input.map(
            move |value| operate(value, target, head),
            engine_state.ctrlc.clone(),
        )
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Apply bits xor to two numbers",
                example: "2 | bits xor 2",
                result: Some(Value::test_int(0)),
            },
            Example {
                description: "Apply logical xor to a list of numbers",
                example: "[8 3 2] | bits xor 2",
                result: Some(Value::List {
                    vals: vec![Value::test_int(10), Value::test_int(1), Value::test_int(0)],
                    span: Span::test_data(),
                }),
            },
        ]
    }
}

fn operate(value: Value, target: i64, head: Span) -> Value {
    match value {
        Value::Int { val, span } => Value::Int {
            val: val ^ target,
            span,
        },
        // Propagate errors by explicitly matching them before the final case.
        Value::Error { .. } => value,
        other => Value::Error {
            error: ShellError::OnlySupportsThisInputType(
                "integer".into(),
                other.get_type().to_string(),
                head,
                // This line requires the Value::Error match above.
                other.expect_span(),
            ),
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
