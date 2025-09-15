use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct Enumerate;

impl Command for Enumerate {
    fn name(&self) -> &str {
        "enumerate"
    }

    fn description(&self) -> &str {
        "Enumerate the elements in a stream."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["itemize"]
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("enumerate")
            .input_output_types(vec![(Type::Any, Type::table())])
            .category(Category::Filters)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Add an index to each element of a list",
            example: r#"[a, b, c] | enumerate "#,
            result: Some(Value::test_list(vec![
                Value::test_record(record! {
                    "index" =>  Value::test_int(0),
                    "item" =>   Value::test_string("a"),
                }),
                Value::test_record(record! {
                    "index" =>  Value::test_int(1),
                    "item" =>   Value::test_string("b"),
                }),
                Value::test_record(record! {
                    "index" =>  Value::test_int(2),
                    "item" =>   Value::test_string("c"),
                }),
            ])),
        }]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let metadata = input.metadata();

        Ok(input
            .into_iter()
            .enumerate()
            .map(move |(idx, x)| {
                Value::record(
                    record! {
                        "index" => Value::int(idx as i64, head),
                        "item" => x,
                    },
                    head,
                )
            })
            .into_pipeline_data_with_metadata(head, engine_state.signals().clone(), metadata))
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
