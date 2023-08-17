use crate::math::reducers::{reducer_for, Reduce};
use crate::math::utils::run_with_function;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Span, SpannedValue, Type,
};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "math avg"
    }

    fn signature(&self) -> Signature {
        Signature::build("math avg")
            .input_output_types(vec![
                (Type::List(Box::new(Type::Number)), Type::Number),
                (Type::List(Box::new(Type::Duration)), Type::Duration),
                (Type::List(Box::new(Type::Filesize)), Type::Filesize),
            ])
            .allow_variants_without_examples(true)
            .category(Category::Math)
    }

    fn usage(&self) -> &str {
        "Returns the average of a list of numbers."
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
    ) -> Result<PipelineData, ShellError> {
        run_with_function(call, input, average)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Compute the average of a list of numbers",
            example: "[-50 100.0 25] | math avg",
            result: Some(SpannedValue::test_float(25.0)),
        }]
    }
}

pub fn average(
    values: &[SpannedValue],
    span: Span,
    head: Span,
) -> Result<SpannedValue, ShellError> {
    let sum = reducer_for(Reduce::Summation);
    let total = &sum(SpannedValue::int(0, head), values.to_vec(), span, head)?;
    match total {
        SpannedValue::Filesize { val, span } => Ok(SpannedValue::Filesize {
            val: val / values.len() as i64,
            span: *span,
        }),
        SpannedValue::Duration { val, span } => Ok(SpannedValue::Duration {
            val: val / values.len() as i64,
            span: *span,
        }),
        _ => total.div(head, &SpannedValue::int(values.len() as i64, head), head),
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
