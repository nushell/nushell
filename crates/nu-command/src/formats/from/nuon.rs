use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct FromNuon;

impl Command for FromNuon {
    fn name(&self) -> &str {
        "from nuon"
    }

    fn usage(&self) -> &str {
        "Convert from nuon to structured data."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("from nuon")
            .input_output_types(vec![(Type::String, Type::Any)])
            .category(Category::Formats)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                example: "'{ a:1 }' | from nuon",
                description: "Converts nuon formatted string to table",
                result: Some(Value::test_record(record! {
                    "a" => Value::test_int(1),
                })),
            },
            Example {
                example: "'{ a:1, b: [1, 2] }' | from nuon",
                description: "Converts nuon formatted string to table",
                result: Some(Value::test_record(record! {
                    "a" => Value::test_int(1),
                    "b" => Value::test_list(vec![Value::test_int(1), Value::test_int(2)]),
                })),
            },
        ]
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let (string_input, _span, metadata) = input.collect_string_strict(head)?;

        match nuon::from_nuon(&string_input, Some(head)) {
            Ok(result) => Ok(result.into_pipeline_data_with_metadata(metadata)),
            Err(err) => Err(ShellError::GenericError {
                error: "error when loading nuon text".into(),
                msg: "could not load nuon text".into(),
                span: Some(head),
                help: None,
                inner: vec![err],
            }),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(FromNuon {})
    }
}
