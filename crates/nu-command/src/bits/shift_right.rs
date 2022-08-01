use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "bits shift-right"
    }

    fn signature(&self) -> Signature {
        Signature::build("bits shift-right")
            .required("bits", SyntaxShape::Int, "number of bits to shift right")
            .category(Category::Bits)
    }

    fn usage(&self) -> &str {
        "Bitwise shift right for integers"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["shr"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let head = call.head;
        let bits: usize = call.req(engine_state, stack, 0)?;

        input.map(
            move |value| operate(value, bits, head),
            engine_state.ctrlc.clone(),
        )
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Shift right a number with 2 bits",
                example: "8 | bits shift-right 2",
                result: Some(Value::Int {
                    val: 2,
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Shift right a list of numbers",
                example: "[15 35 2] | bits shift-right 2",
                result: Some(Value::List {
                    vals: vec![Value::test_int(3), Value::test_int(8), Value::test_int(0)],
                    span: Span::test_data(),
                }),
            },
        ]
    }
}

fn operate(value: Value, bits: usize, head: Span) -> Value {
    match value {
        Value::Int { val, span } => {
            let shift_bits = (bits % 64) as u32;
            match val.checked_shr(shift_bits) {
                Some(val) => Value::Int { val, span },
                None => Value::Error {
                    error: ShellError::GenericError(
                        "Shift right overflow".to_string(),
                        format!("{} shift right {} bits will be overflow", val, shift_bits),
                        Some(span),
                        None,
                        Vec::new(),
                    ),
                },
            }
        }
        other => Value::Error {
            error: ShellError::UnsupportedInput(
                format!(
                    "Only integer values are supported, input type: {:?}",
                    other.get_type()
                ),
                other.span().unwrap_or(head),
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
