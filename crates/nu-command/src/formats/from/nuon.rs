use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct FromNuon;

impl Command for FromNuon {
    fn name(&self) -> &str {
        "from nuon"
    }

    fn description(&self) -> &str {
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
            Example {
                example: "'{a:1,b:[1,2]}' | from nuon",
                description: "Converts raw nuon formatted string to table",
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
            Ok(result) => Ok(result
                .into_pipeline_data_with_metadata(metadata.map(|md| md.with_content_type(None)))),
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
    use nu_cmd_lang::eval_pipeline_without_terminal_expression;

    use crate::Reject;
    use crate::{Metadata, MetadataSet};

    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(FromNuon {})
    }

    #[test]
    fn test_content_type_metadata() {
        let mut engine_state = Box::new(EngineState::new());
        let delta = {
            let mut working_set = StateWorkingSet::new(&engine_state);

            working_set.add_decl(Box::new(FromNuon {}));
            working_set.add_decl(Box::new(Metadata {}));
            working_set.add_decl(Box::new(MetadataSet {}));
            working_set.add_decl(Box::new(Reject {}));

            working_set.render()
        };

        engine_state
            .merge_delta(delta)
            .expect("Error merging delta");

        let cmd = r#"'[[a, b]; [1, 2]]' | metadata set --content-type 'application/x-nuon' --datasource-ls | from nuon | metadata | reject span | $in"#;
        let result = eval_pipeline_without_terminal_expression(
            cmd,
            std::env::temp_dir().as_ref(),
            &mut engine_state,
        );
        assert_eq!(
            Value::test_record(record!("source" => Value::test_string("ls"))),
            result.expect("There should be a result")
        )
    }
}
