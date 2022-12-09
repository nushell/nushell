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
        "math arctan2"
    }

    fn signature(&self) -> Signature {
        Signature::build("math arctan2")
            .switch("degrees", "Return degrees instead of radians", Some('d'))
            .input_output_types(vec![(Type::Number, Type::Float)])
            .vectorizes_over_list(true)
            .required(
                "target",
                SyntaxShape::Number,
                "target number to compute arctan2 with",
            )
            .category(Category::Math)
    }

    fn usage(&self) -> &str {
        "Returns the four quadrant arctangent of two numbers."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["trigonometry", "arc-tangent2"]
    }

    fn examples(&self) -> Vec<Example> {
        let result1 = 42.0_f64.atan2(10.0_f64);
        let result2: Vec<Value> = vec![2.0_f64, 5.0_f64, 4.0_f64]
            .into_iter()
            .map(|n| n.atan2(7.0_f64))
            .map(Value::test_float)
            .collect();

        vec![
            Example {
                description: "Compute the arctan2 of two numbers",
                example: "42 | math arctan2 10",
                result: Some(Value::test_float(result1)),
            },
            Example {
                description: "Compute the arctan2 of each number in a list and a given number",
                example: "[2 5 4] | math arctan2 7",
                result: Some(Value::List {
                    vals: result2,
                    span: Span::test_data(),
                }),
            },
        ]
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
        let use_degrees = call.has_flag("degrees");

        input.map(
            move |value| operate(value, target, head, use_degrees),
            engine_state.ctrlc.clone(),
        )
    }
}

fn operate(value: Value, target: i64, head: Span, use_degrees: bool) -> Value {
    match value {
        numeric @ (Value::Int { .. } | Value::Float { .. }) => {
            let (val, span) = match numeric {
                Value::Int { val, span } => (val as f64, span),
                Value::Float { val, span } => (val, span),
                _ => unreachable!(),
            };

            let val = val.atan2(target as f64);
            let val = if use_degrees { val.to_degrees() } else { val };

            Value::Float { val, span }
        }

        other => Value::Error {
            error: ShellError::UnsupportedInput(
                format!(
                    "Only numerical values are supported, input type: {:?}",
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
