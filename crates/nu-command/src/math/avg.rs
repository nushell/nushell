use crate::math::reducers::{reducer_for, Reduce};
use crate::math::utils::run_with_function;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, PipelineData, ShellError, Signature, Span, Type, Value};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "math avg"
    }

    fn signature(&self) -> Signature {
        Signature::build("math avg")
            .input_output_types(vec![(Type::List(Box::new(Type::Number)), Type::Number)])
            .category(Category::Math)
    }

    fn usage(&self) -> &str {
        "Returns the average of a list of numbers"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["average", "mean", "statistics"]
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        run_with_function(call, input, average)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Compute the average of a list of numbers",
            example: "[-50 100.0 25] | math avg",
            result: Some(Value::Float {
                val: 25.0,
                span: Span::test_data(),
            }),
        }]
    }
}

pub fn average(values: &[Value], head: &Span) -> Result<Value, ShellError> {
    let sum = reducer_for(Reduce::Summation);
    let total = &sum(
        Value::Int {
            val: 0,
            span: *head,
        },
        values.to_vec(),
        *head,
    )?;
    match total {
        Value::Filesize { val, span } => Ok(Value::Filesize {
            val: val / values.len() as i64,
            span: *span,
        }),
        Value::Duration { val, span } => Ok(Value::Duration {
            val: val / values.len() as i64,
            span: *span,
        }),
        _ => total.div(
            *head,
            &Value::Int {
                val: values.len() as i64,
                span: *head,
            },
            *head,
        ),
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
