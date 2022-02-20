use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, Range, ShellError, Signature, Span, SyntaxShape, Value,
};
use rand::prelude::{thread_rng, Rng};
use std::cmp::Ordering;

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "random decimal"
    }

    fn signature(&self) -> Signature {
        Signature::build("random decimal")
            .optional("range", SyntaxShape::Range, "Range of values")
            .category(Category::Random)
    }

    fn usage(&self) -> &str {
        "Generate a random decimal within a range [min..max]"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        decimal(engine_state, stack, call)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Generate a default decimal value between 0 and 1",
                example: "random decimal",
                result: None,
            },
            Example {
                description: "Generate a random decimal less than or equal to 500",
                example: "random decimal ..500",
                result: None,
            },
            Example {
                description: "Generate a random decimal greater than or equal to 100000",
                example: "random decimal 100000..",
                result: None,
            },
            Example {
                description: "Generate a random decimal between 1.0 and 1.1",
                example: "random decimal 1.0..1.1",
                result: None,
            },
        ]
    }
}

fn decimal(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<PipelineData, ShellError> {
    let span = call.head;
    let range: Option<Range> = call.opt(engine_state, stack, 0)?;

    let (min, max) = if let Some(r) = range {
        if r.is_end_inclusive() {
            (r.from.as_float()?, r.to.as_float()?)
        } else if r.to.as_float()? >= 1.0 {
            (r.from.as_float()?, r.to.as_float()? - 1.0)
        } else {
            (0.0, 0.0)
        }
    } else {
        (0.0, 1.0)
    };

    match min.partial_cmp(&max) {
        Some(Ordering::Greater) => Err(ShellError::InvalidRange(
            min.to_string(),
            max.to_string(),
            span,
        )),
        Some(Ordering::Equal) => Ok(PipelineData::Value(
            Value::Float {
                val: min,
                span: Span::new(64, 64),
            },
            None,
        )),
        _ => {
            let mut thread_rng = thread_rng();
            let result: f64 = thread_rng.gen_range(min..max);

            Ok(PipelineData::Value(
                Value::Float {
                    val: result,
                    span: Span::new(64, 64),
                },
                None,
            ))
        }
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
