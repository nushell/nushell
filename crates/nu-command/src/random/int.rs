use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, Range, ShellError, Signature, Spanned, SyntaxShape, Type,
    Value,
};
use rand::prelude::{thread_rng, Rng};
use std::cmp::Ordering;

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "random int"
    }

    fn signature(&self) -> Signature {
        Signature::build("random int")
            .input_output_types(vec![(Type::Nothing, Type::Int)])
            .allow_variants_without_examples(true)
            .optional("range", SyntaxShape::Range, "Range of values.")
            .category(Category::Random)
    }

    fn usage(&self) -> &str {
        "Generate a random integer [min..max]."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["generate", "natural", "number"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        integer(engine_state, stack, call)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Generate an unconstrained random integer",
                example: "random int",
                result: None,
            },
            Example {
                description: "Generate a random integer less than or equal to 500",
                example: "random int ..500",
                result: None,
            },
            Example {
                description: "Generate a random integer greater than or equal to 100000",
                example: "random int 100000..",
                result: None,
            },
            Example {
                description: "Generate a random integer between 1 and 10",
                example: "random int 1..10",
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
    let range: Option<Spanned<Range>> = call.opt(engine_state, stack, 0)?;

    let mut range_span = call.head;
    let (min, max) = if let Some(spanned_range) = range {
        let r = spanned_range.item;
        range_span = spanned_range.span;
        if r.is_end_inclusive() {
            (r.from.as_int()?, r.to.as_int()?)
        } else if r.to.as_int()? > 0 {
            (r.from.as_int()?, r.to.as_int()? - 1)
        } else {
            (0, 0)
        }
    } else {
        (0, i64::MAX)
    };

    match min.partial_cmp(&max) {
        Some(Ordering::Greater) => Err(ShellError::InvalidRange {
            left_flank: min.to_string(),
            right_flank: max.to_string(),
            span: range_span,
        }),
        Some(Ordering::Equal) => Ok(PipelineData::Value(Value::int(min, span), None)),
        _ => {
            let mut thread_rng = thread_rng();
            let result: i64 = thread_rng.gen_range(min..=max);

            Ok(PipelineData::Value(Value::int(result, span), None))
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
