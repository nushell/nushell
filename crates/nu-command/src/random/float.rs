use nu_engine::command_prelude::*;
use nu_protocol::{FloatRange, Range};
use rand::random_range;
use std::ops::Bound;

#[derive(Clone)]
pub struct RandomFloat;

impl Command for RandomFloat {
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

    fn description(&self) -> &str {
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

    fn examples(&self) -> Vec<Example<'_>> {
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
    let span = call.head;
    let range: Option<Spanned<Range>> = call.opt(engine_state, stack, 0)?;

    match range {
        Some(range) => {
            let range_span = range.span;
            let range = FloatRange::from(range.item);

            if range.step() < 0.0 {
                return Err(ShellError::InvalidRange {
                    left_flank: range.start().to_string(),
                    right_flank: match range.end() {
                        Bound::Included(end) | Bound::Excluded(end) => end.to_string(),
                        Bound::Unbounded => "".into(),
                    },
                    span: range_span,
                });
            }

            let value = match range.end() {
                Bound::Included(end) => random_range(range.start()..=end),
                Bound::Excluded(end) => random_range(range.start()..end),
                Bound::Unbounded => random_range(range.start()..f64::MAX),
            };

            Ok(PipelineData::value(Value::float(value, span), None))
        }
        None => Ok(PipelineData::value(
            Value::float(random_range(0.0..1.0), span),
            None,
        )),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(RandomFloat {})
    }
}
