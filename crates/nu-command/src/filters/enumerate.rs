use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    record, Category, Example, IntoInterruptiblePipelineData, PipelineData, Record, ShellError,
    Signature, Span, Type, Value,
};

#[derive(Clone)]
pub struct Enumerate;

impl Command for Enumerate {
    fn name(&self) -> &str {
        "enumerate"
    }

    fn usage(&self) -> &str {
        "Enumerate the elements in a stream."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["itemize"]
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("enumerate")
            .input_output_types(vec![(Type::Any, Type::Any)])
            .category(Category::Filters)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Add an index to each element of a list",
            example: r#"[a, b, c] | enumerate "#,
            result: Some(Value::List {
                vals: vec![
                    Value::test_record(Record {
                        cols: vec!["index".into(), "item".into()],
                        vals: vec![Value::test_int(0), Value::test_string("a")],
                    }),
                    Value::test_record(Record {
                        cols: vec!["index".into(), "item".into()],
                        vals: vec![Value::test_int(1), Value::test_string("b")],
                    }),
                    Value::test_record(Record {
                        cols: vec!["index".into(), "item".into()],
                        vals: vec![Value::test_int(2), Value::test_string("c")],
                    }),
                ],
                span: Span::test_data(),
            }),
        }]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let metadata = input.metadata();
        let ctrlc = engine_state.ctrlc.clone();
        let span = call.head;

        Ok(input
            .into_iter()
            .enumerate()
            .map(move |(idx, x)| {
                Value::record(
                    record! {
                        index => Value::int(idx as i64, span),
                        item => x,
                    },
                    span,
                )
            })
            .into_pipeline_data_with_metadata(metadata, ctrlc))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Enumerate {})
    }
}
