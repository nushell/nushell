use super::delimited::{DelimitedReaderConfig, from_delimited_data, trim_from_str};
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct FromTsv;

impl Command for FromTsv {
    fn name(&self) -> &str {
        "from tsv"
    }

    fn signature(&self) -> Signature {
        Signature::build("from tsv")
            .input_output_types(vec![(Type::String, Type::table())])
            .named(
                "comment",
                SyntaxShape::String,
                "a comment character to ignore lines starting with it",
                Some('c'),
            )
            .named(
                "quote",
                SyntaxShape::String,
                "a quote character to ignore separators in strings, defaults to '\"'",
                Some('q'),
            )
            .named(
                "escape",
                SyntaxShape::String,
                "an escape character for strings containing the quote character",
                Some('e'),
            )
            .switch(
                "noheaders",
                "don't treat the first row as column names",
                Some('n'),
            )
            .switch(
                "flexible",
                "allow the number of fields in records to be variable",
                None,
            )
            .switch("no-infer", "no field type inferencing", None)
            .param(
                Flag::new("trim")
                    .short('t')
                    .arg(SyntaxShape::String)
                    .desc(
                        "drop leading and trailing whitespaces around headers names and/or field \
                         values",
                    )
                    .completion(Completion::new_list(&["all", "fields", "headers", "none"])),
            )
            .category(Category::Formats)
    }

    fn description(&self) -> &str {
        "Parse text as .tsv and create table."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        from_tsv(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Convert tab-separated data to a table",
                example: "\"ColA\tColB\n1\t2\" | from tsv",
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "ColA" =>  Value::test_int(1),
                    "ColB" =>  Value::test_int(2),
                })])),
            },
            Example {
                description: "Convert comma-separated data to a table, allowing variable number of columns per row and ignoring headers",
                example: "\"value 1\nvalue 2\tdescription 2\" | from tsv --flexible --noheaders",
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "column0" => Value::test_string("value 1"),
                    }),
                    Value::test_record(record! {
                        "column0" => Value::test_string("value 2"),
                        "column1" => Value::test_string("description 2"),
                    }),
                ])),
            },
            Example {
                description: "Create a tsv file with header columns and open it",
                example: r#"$'c1(char tab)c2(char tab)c3(char nl)1(char tab)2(char tab)3' | save tsv-data | open tsv-data | from tsv"#,
                result: None,
            },
            Example {
                description: "Create a tsv file without header columns and open it",
                example: r#"$'a1(char tab)b1(char tab)c1(char nl)a2(char tab)b2(char tab)c2' | save tsv-data | open tsv-data | from tsv --noheaders"#,
                result: None,
            },
            Example {
                description: "Create a tsv file without header columns and open it, removing all unnecessary whitespaces",
                example: r#"$'a1(char tab)b1(char tab)c1(char nl)a2(char tab)b2(char tab)c2' | save tsv-data | open tsv-data | from tsv --trim all"#,
                result: None,
            },
            Example {
                description: "Create a tsv file without header columns and open it, removing all unnecessary whitespaces in the header names",
                example: r#"$'a1(char tab)b1(char tab)c1(char nl)a2(char tab)b2(char tab)c2' | save tsv-data | open tsv-data | from tsv --trim headers"#,
                result: None,
            },
            Example {
                description: "Create a tsv file without header columns and open it, removing all unnecessary whitespaces in the field values",
                example: r#"$'a1(char tab)b1(char tab)c1(char nl)a2(char tab)b2(char tab)c2' | save tsv-data | open tsv-data | from tsv --trim fields"#,
                result: None,
            },
        ]
    }
}

fn from_tsv(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let name = call.head;

    let comment = call
        .get_flag(engine_state, stack, "comment")?
        .map(|v: Value| v.as_char())
        .transpose()?;
    let quote = call
        .get_flag(engine_state, stack, "quote")?
        .map(|v: Value| v.as_char())
        .transpose()?
        .unwrap_or('"');
    let escape = call
        .get_flag(engine_state, stack, "escape")?
        .map(|v: Value| v.as_char())
        .transpose()?;
    let no_infer = call.has_flag(engine_state, stack, "no-infer")?;
    let noheaders = call.has_flag(engine_state, stack, "noheaders")?;
    let flexible = call.has_flag(engine_state, stack, "flexible")?;
    let trim = trim_from_str(call.get_flag(engine_state, stack, "trim")?)?;

    let config = DelimitedReaderConfig {
        separator: '\t',
        comment,
        quote,
        escape,
        noheaders,
        flexible,
        no_infer,
        trim,
    };

    from_delimited_data(config, input, name)
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

        test_examples(FromTsv {})
    }

    #[test]
    fn test_content_type_metadata() {
        let mut engine_state = Box::new(EngineState::new());
        let delta = {
            let mut working_set = StateWorkingSet::new(&engine_state);

            working_set.add_decl(Box::new(FromTsv {}));
            working_set.add_decl(Box::new(Metadata {}));
            working_set.add_decl(Box::new(MetadataSet {}));
            working_set.add_decl(Box::new(Reject {}));

            working_set.render()
        };

        engine_state
            .merge_delta(delta)
            .expect("Error merging delta");

        let cmd = r#""a\tb\n1\t2" | metadata set --content-type 'text/tab-separated-values' --datasource-ls | from tsv | metadata | reject span | $in"#;
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
