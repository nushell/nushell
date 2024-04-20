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
                "remove all of the whitespace (default behaviour and overwrites -i and -t)",
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
            .category(Category::Formats)
    }

    fn usage(&self) -> &str {
        "Converts table data into Nuon (Nushell Object Notation) text."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let raw_flag = call.has_flag(engine_state, stack, "raw")?;
        let tabs_flag = call.get_flag(engine_state, stack, "tabs")?;
        let indent_flag = call.get_flag(engine_state, stack, "indent")?;
        let style = match (raw_flag, tabs_flag, indent_flag) {
            (true, None, None) => nuon::ToStyle::Raw,
            (false, None, None) => nuon::ToStyle::Raw,
            (false, None, Some(indent)) => nuon::ToStyle::Spaces(indent),
            (false, Some(n_tabs), None) => nuon::ToStyle::Tabs(n_tabs),
            (true, None, Some(_)) => {
                return Err(ShellError::IncompatibleParameters {
                    left_message: "can't use `--indent` at the same time".into(),
                    left_span: call.get_named_arg("indent").expect("has flag").span,
                    right_message: "because of `--raw`".into(),
                    right_span: call.get_named_arg("raw").expect("has flag").span,
                });
            }
            (true, Some(_), _) => {
                return Err(ShellError::IncompatibleParameters {
                    left_message: "can't use `--tabs` at the same time".into(),
                    left_span: call.get_named_arg("tabs").expect("has flag").span,
                    right_message: "because of `--raw`".into(),
                    right_span: call.get_named_arg("raw").expect("has flag").span,
                });
            }
            (false, Some(_), Some(_)) => {
                return Err(ShellError::IncompatibleParameters {
                    left_message: "can't use `--indent` at the same time".into(),
                    left_span: call.get_named_arg("indent").expect("has flag").span,
                    right_message: "because of `--tabs`".into(),
                    right_span: call.get_named_arg("tabs").expect("has flag").span,
                });
            }
        };

        let span = call.head;
        let value = input.into_value(span);

        match nuon::to_nuon(&value, style, Some(span)) {
            Ok(serde_nuon_string) => {
                Ok(Value::string(serde_nuon_string, span).into_pipeline_data())
            }
            _ => Ok(Value::error(
                ShellError::CantConvert {
                    to_type: "NUON".into(),
                    from_type: value.get_type().to_string(),
                    span,
                    help: None,
                },
                span,
            )
            .into_pipeline_data()),
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Outputs a NUON string representing the contents of this list, compact by default",
                example: "[1 2 3] | to nuon",
                result: Some(Value::test_string("[1, 2, 3]"))
            },
            Example {
                description: "Outputs a NUON array of ints, with pretty indentation",
                example: "[1 2 3] | to nuon --indent 2",
                result: Some(Value::test_string("[\n  1,\n  2,\n  3\n]")),
            },
            Example {
                description: "A more complex record with multiple data types",
                example: "{date: 2000-01-01, data: [1 [2 3] 4.56]} | to nuon --indent 2",
                result: Some(Value::test_string("{\n  date: 2000-01-01T00:00:00+00:00,\n  data: [\n    1,\n    [\n      2,\n      3\n    ],\n    4.56\n  ]\n}"))
            }
        ]
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test_examples() {
        use super::ToNuon;
        use crate::test_examples;
        test_examples(ToNuon {})
    }
}
