use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct Reverse;

impl Command for Reverse {
    fn name(&self) -> &str {
        "reverse"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("reverse")
            .input_output_types(vec![(
                Type::List(Box::new(Type::Any)),
                Type::List(Box::new(Type::Any)),
            )])
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Reverses the input list or table."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["convert, inverse, flip"]
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                example: "[0,1,2,3] | reverse",
                description: "Reverse a list",
                result: Some(Value::test_list(vec![
                    Value::test_int(3),
                    Value::test_int(2),
                    Value::test_int(1),
                    Value::test_int(0),
                ])),
            },
            Example {
                example: "[{a: 1} {a: 2}] | reverse",
                description: "Reverse a table",
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "a" => Value::test_int(2),
                    }),
                    Value::test_record(record! {
                        "a" => Value::test_int(1),
                    }),
                ])),
            },
        ]
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
        let values = input.into_iter_strict(head)?.collect::<Vec<_>>();
        let iter = values.into_iter().rev();
        Ok(iter.into_pipeline_data_with_metadata(head, engine_state.signals().clone(), metadata))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Reverse {})
    }
}
