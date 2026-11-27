use nu_engine::command_prelude::*;
use nu_protocol::{FloatRange, Range};
use rand::{distr::uniform::SampleRange, random_range};
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
                description: "Generate a default float value between 0 and 1.",
                example: "random float",
                result: None,
            },
            Example {
                description: "Generate a random float less than or equal to 500.",
                example: "random float ..500",
                result: None,
            },
            Example {
                description: "Generate a random float greater than or equal to 100000.",
                example: "random float 100000..",
                result: None,
            },
            Example {
                description: "Generate a random float between 1.0 and 1.1.",
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
            let start = range.start();

            let invalid_range = || ShellError::InvalidRange {
                left_flank: start.to_string(),
                right_flank: match range.end() {
                    Bound::Included(end) => end.to_string(),
                    Bound::Excluded(end) => format!("<{end}"),
                    Bound::Unbounded => "".into(),
                },
                span: range_span,
            };

            if range.step() < 0.0 {
                return Err(invalid_range());
            }

            let mut rng = rand::rng();
            let value = match range.end() {
                Bound::Included(end) => (start..=end).sample_single(&mut rng),
                Bound::Excluded(end) if start < end => (start..end).sample_single(&mut rng),
                Bound::Unbounded if start < f64::MAX => (start..f64::MAX).sample_single(&mut rng),
                Bound::Excluded(_) | Bound::Unbounded => return Err(invalid_range()),
            }
            .map_err(|_| invalid_range())?;

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
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(RandomFloat)
    }
}
