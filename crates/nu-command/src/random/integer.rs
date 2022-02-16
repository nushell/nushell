use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, Range, ShellError, Signature, SyntaxShape, Value,
};
use rand::prelude::{thread_rng, Rng};
use std::cmp::Ordering;

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "random integer"
    }

    fn signature(&self) -> Signature {
        Signature::build("random integer")
            .optional("range", SyntaxShape::Range, "Range of values")
            .category(Category::Random)
    }

    fn usage(&self) -> &str {
        "Generate a random integer [min..max]"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        integer(engine_state, stack, call)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Generate an unconstrained random integer",
                example: "random integer",
                result: None,
            },
            Example {
                description: "Generate a random integer less than or equal to 500",
                example: "random integer ..500",
                result: None,
            },
            Example {
                description: "Generate a random integer greater than or equal to 100000",
                example: "random integer 100000..",
                result: None,
            },
            Example {
                description: "Generate a random integer between 1 and 10",
                example: "random integer 1..10",
                result: None,
            },
        ]
    }
}

fn integer(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<PipelineData, ShellError> {
    let span = call.head;
    let range: Option<Range> = call.opt(engine_state, stack, 0)?;

    let (min, max) = if let Some(r) = range {
        if r.is_end_inclusive() {
            (r.from.as_integer()?, r.to.as_integer()?)
        } else if r.to.as_integer()? > 0 {
            (r.from.as_integer()?, r.to.as_integer()? - 1)
        } else {
            (0, 0)
        }
    } else {
        (0, i64::MAX)
    };

    match min.partial_cmp(&max) {
        Some(Ordering::Greater) => Err(ShellError::InvalidRange(
            min.to_string(),
            max.to_string(),
            span,
        )),
        Some(Ordering::Equal) => Ok(PipelineData::Value(Value::Int { val: min, span }, None)),
        _ => {
            let mut thread_rng = thread_rng();
            let result: i64 = thread_rng.gen_range(min..=max);

            Ok(PipelineData::Value(Value::Int { val: result, span }, None))
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
