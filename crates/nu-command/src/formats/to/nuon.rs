use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct ToNuon;

impl Command for ToNuon {
    fn name(&self) -> &str {
        "to nuon"
    }

    fn signature(&self) -> Signature {
        Signature::build("to nuon")
            .input_output_types(vec![(Type::Any, Type::String)])
            .switch(
                "raw",
                "remove all of the whitespace (overwrites -i and -t)",
                Some('r'),
            )
            .named(
                "indent",
                SyntaxShape::Number,
                "specify indentation width",
                Some('i'),
            )
            .named(
                "tabs",
                SyntaxShape::Number,
                "specify indentation tab quantity",
                Some('t'),
            )
            .switch(
                "serialize",
                "serialize nushell types that cannot be deserialized",
                Some('s'),
            )
            .category(Category::Formats)
    }

    fn description(&self) -> &str {
        "Converts table data into Nuon (Nushell Object Notation) text."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let metadata = input
            .metadata()
            .unwrap_or_default()
            .with_content_type(Some("application/x-nuon".into()));

        let serialize_types = call.has_flag(engine_state, stack, "serialize")?;
        let style = if call.has_flag(engine_state, stack, "raw")? {
            nuon::ToStyle::Raw
        } else if let Some(t) = call.get_flag(engine_state, stack, "tabs")? {
            nuon::ToStyle::Tabs(t)
        } else if let Some(i) = call.get_flag(engine_state, stack, "indent")? {
            nuon::ToStyle::Spaces(i)
        } else {
            nuon::ToStyle::Default
        };

        let span = call.head;
        let value = input.into_value(span)?;

        match nuon::to_nuon(engine_state, &value, style, Some(span), serialize_types) {
            Ok(serde_nuon_string) => Ok(Value::string(serde_nuon_string, span)
                .into_pipeline_data_with_metadata(Some(metadata))),
            Err(error) => {
                Ok(Value::error(error, span).into_pipeline_data_with_metadata(Some(metadata)))
            }
        }
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Outputs a NUON string representing the contents of this list, compact by default",
                example: "[1 2 3] | to nuon",
                result: Some(Value::test_string("[1, 2, 3]")),
            },
            Example {
                description: "Outputs a NUON array of ints, with pretty indentation",
                example: "[1 2 3] | to nuon --indent 2",
                result: Some(Value::test_string("[\n  1,\n  2,\n  3\n]")),
            },
            Example {
                description: "Overwrite any set option with --raw",
                example: "[1 2 3] | to nuon --indent 2 --raw",
                result: Some(Value::test_string("[1,2,3]")),
            },
            Example {
                description: "A more complex record with multiple data types",
                example: "{date: 2000-01-01, data: [1 [2 3] 4.56]} | to nuon --indent 2",
                result: Some(Value::test_string(
                    "{\n  date: 2000-01-01T00:00:00+00:00,\n  data: [\n    1,\n    [\n      2,\n      3\n    ],\n    4.56\n  ]\n}",
                )),
            },
            Example {
                description: "A more complex record with --raw",
                example: "{date: 2000-01-01, data: [1 [2 3] 4.56]} | to nuon --raw",
                result: Some(Value::test_string(
                    "{date:2000-01-01T00:00:00+00:00,data:[1,[2,3],4.56]}",
                )),
            },
        ]
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use nu_cmd_lang::eval_pipeline_without_terminal_expression;

    use crate::{Get, Metadata};

    #[test]
    fn test_examples() {
        use super::ToNuon;
        use crate::test_examples;
        test_examples(ToNuon {})
    }

    #[test]
    fn test_content_type_metadata() {
        let mut engine_state = Box::new(EngineState::new());
        let delta = {
            // Base functions that are needed for testing
            // Try to keep this working set small to keep tests running as fast as possible
            let mut working_set = StateWorkingSet::new(&engine_state);

            working_set.add_decl(Box::new(ToNuon {}));
            working_set.add_decl(Box::new(Metadata {}));
            working_set.add_decl(Box::new(Get {}));

            working_set.render()
        };

        engine_state
            .merge_delta(delta)
            .expect("Error merging delta");

        let cmd = "{a: 1 b: 2} | to nuon | metadata | get content_type | $in";
        let result = eval_pipeline_without_terminal_expression(
            cmd,
            std::env::temp_dir().as_ref(),
            &mut engine_state,
        );
        assert_eq!(
            Value::test_string("application/x-nuon"),
            result.expect("There should be a result")
        );
    }
}
