use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, Range, ShellError, Signature, Span, Spanned, SyntaxShape,
    Type, Value,
};
use rand::prelude::{thread_rng, Rng};
use std::cmp::Ordering;

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "random float"
    }

    fn signature(&self) -> Signature {
        Signature::build("random float")
            .input_output_types(vec![(Type::Nothing, Type::Float)])
            .allow_variants_without_examples(true)
            .optional("range", SyntaxShape::Range, "Range of values.")
            .category(Category::Random)
    }

    fn usage(&self) -> &str {
        "Generate a random float within a range [min..max]."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["generate"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        float(engine_state, stack, call)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Generate a default float value between 0 and 1",
                example: "random float",
                result: None,
            },
            Example {
                description: "Generate a random float less than or equal to 500",
                example: "random float ..500",
                result: None,
            },
            Example {
                description: "Generate a random float greater than or equal to 100000",
                example: "random float 100000..",
                result: None,
            },
            Example {
                description: "Generate a random float between 1.0 and 1.1",
                example: "random float 1.0..1.1",
                result: None,
            },
        ]
    }
}

fn float(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<PipelineData, ShellError> {
    let mut range_span = call.head;
    let range: Option<Spanned<Range>> = call.opt(engine_state, stack, 0)?;

    let (min, max) = if let Some(spanned_range) = range {
        let r = spanned_range.item;
        range_span = spanned_range.span;

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
        Some(Ordering::Greater) => Err(ShellError::InvalidRange {
            left_flank: min.to_string(),
            right_flank: max.to_string(),
            span: range_span,
        }),
        Some(Ordering::Equal) => Ok(PipelineData::Value(
            Value::float(min, Span::new(64, 64)),
            None,
        )),
        _ => {
            let mut thread_rng = thread_rng();
            let result: f64 = thread_rng.gen_range(min..max);

            Ok(PipelineData::Value(
                Value::float(result, Span::new(64, 64)),
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
