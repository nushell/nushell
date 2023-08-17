use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoInterruptiblePipelineData, IntoPipelineData, PipelineData, ShellError,
    Signature, Span, SpannedValue, SyntaxShape, Type,
};

#[derive(Clone)]
pub struct Zip;

impl Command for Zip {
    fn name(&self) -> &str {
        "zip"
    }

    fn usage(&self) -> &str {
        "Combine a stream with the input."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("zip")
            .input_output_types(vec![
                (
                    Type::List(Box::new(Type::Any)),
                    Type::List(Box::new(Type::List(Box::new(Type::Any)))),
                ),
                (
                    Type::Range,
                    Type::List(Box::new(Type::List(Box::new(Type::Any)))),
                ),
            ])
            .required("other", SyntaxShape::Any, "the other input")
            .category(Category::Filters)
    }

    fn examples(&self) -> Vec<Example> {
        let test_row_1 = SpannedValue::List {
            vals: vec![SpannedValue::test_int(1), SpannedValue::test_int(4)],
            span: Span::test_data(),
        };

        let test_row_2 = SpannedValue::List {
            vals: vec![SpannedValue::test_int(2), SpannedValue::test_int(5)],
            span: Span::test_data(),
        };

        let test_row_3 = SpannedValue::List {
            vals: vec![SpannedValue::test_int(3), SpannedValue::test_int(6)],
            span: Span::test_data(),
        };

        vec![
            Example {
                example: "[1 2] | zip [3 4]",
                description: "Zip two lists",
                result: Some(SpannedValue::List {
                    vals: vec![
                        SpannedValue::List {
                            vals: vec![SpannedValue::test_int(1), SpannedValue::test_int(3)],
                            span: Span::test_data(),
                        },
                        SpannedValue::List {
                            vals: vec![SpannedValue::test_int(2), SpannedValue::test_int(4)],
                            span: Span::test_data(),
                        },
                    ],
                    span: Span::test_data(),
                }),
            },
            Example {
                example: "1..3 | zip 4..6",
                description: "Zip two ranges",
                result: Some(SpannedValue::List {
                    vals: vec![test_row_1, test_row_2, test_row_3],
                    span: Span::test_data(),
                }),
            },
            Example {
                example: "glob *.ogg | zip ['bang.ogg', 'fanfare.ogg', 'laser.ogg'] | each {|| mv $in.0 $in.1 }",
                description: "Rename .ogg files to match an existing list of filenames",
                result: None,
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
        let other: SpannedValue = call.req(engine_state, stack, 0)?;
        let head = call.head;
        let ctrlc = engine_state.ctrlc.clone();
        let metadata = input.metadata();

        Ok(input
            .into_iter()
            .zip(other.into_pipeline_data())
            .map(move |(x, y)| SpannedValue::List {
                vals: vec![x, y],
                span: head,
            })
            .into_pipeline_data(ctrlc)
            .set_metadata(metadata))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Zip {})
    }
}
