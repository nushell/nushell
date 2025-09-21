use nu_engine::command_prelude::*;
use nu_protocol::Range;
use rand::random_range;
use std::ops::Bound;

#[derive(Clone)]
pub struct RandomInt;

impl Command for RandomInt {
    fn name(&self) -> &str {
        "random int"
    }

    fn signature(&self) -> Signature {
        Signature::build("random int")
            .input_output_types(vec![(Type::Nothing, Type::Int)])
            .allow_variants_without_examples(true)
            .optional(
                "range",
                SyntaxShape::Range,
                "Range of potential values, inclusive of both start and end values.",
            )
            .category(Category::Random)
    }

    fn description(&self) -> &str {
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

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Generate a non-negative random integer",
                example: "random int",
                result: None,
            },
            Example {
                description: "Generate a random integer between 0 (inclusive) and 500 (inclusive)",
                example: "random int ..500",
                result: None,
            },
            Example {
                description: "Generate a random integer greater than or equal to 100000",
                example: "random int 100000..",
                result: None,
            },
            Example {
                description: "Generate a random integer between -10 (inclusive) and 10 (inclusive)",
                example: "random int (-10)..10",
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

    match range {
        Some(range) => {
            let range_span = range.span;
            match range.item {
                Range::IntRange(range) => {
                    if range.step() < 0 {
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
                        Bound::Unbounded => random_range(range.start()..=i64::MAX),
                    };

                    Ok(PipelineData::value(Value::int(value, span), None))
                }
                Range::FloatRange(_) => Err(ShellError::UnsupportedInput {
                    msg: "float range".into(),
                    input: "value originates from here".into(),
                    msg_span: call.head,
                    input_span: range.span,
                }),
            }
        }
        None => Ok(PipelineData::value(
            Value::int(random_range(0..=i64::MAX), span),
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

        test_examples(RandomInt {})
    }
}
