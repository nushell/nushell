use super::delimited::{DelimitedReaderConfig, from_delimited_data, trim_from_str};
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct FromCsv;

impl Command for FromCsv {
    fn name(&self) -> &str {
        "from csv"
    }

    fn signature(&self) -> Signature {
        Signature::build("from csv")
            .input_output_types(vec![
                (Type::String, Type::table()),
            ])
            .named(
                "separator",
                SyntaxShape::String,
                "A character to separate columns (either single char or 4 byte unicode sequence), defaults to ','.",
                Some('s'),
            )
            .named(
                "comment",
                SyntaxShape::String,
                "A comment character to ignore lines starting with it.",
                Some('c'),
            )
            .named(
                "quote",
                SyntaxShape::String,
                "A quote character to ignore separators in strings, defaults to '\"'.",
                Some('q'),
            )
            .named(
                "escape",
                SyntaxShape::String,
                "An escape character for strings containing the quote character.",
                Some('e'),
            )
            .switch(
                "noheaders",
                "Don't treat the first row as column names.",
                Some('n'),
            )
            .switch(
                "flexible",
                "Allow the number of fields in records to be variable.",
                None,
            )
            .switch("no-infer", "No field type inferencing.", None)
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
        "Parse text as .csv and create table."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        from_csv(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Convert comma-separated data to a table.",
                example: "\"ColA,ColB\n1,2\" | from csv",
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "ColA" => Value::test_int(1),
                    "ColB" => Value::test_int(2),
                })])),
            },
            Example {
                description: "Convert comma-separated data to a table, allowing variable number of columns per row.",
                example: "\"ColA,ColB\n1,2\n3,4,5\n6\" | from csv --flexible",
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "ColA" => Value::test_int(1),
                        "ColB" => Value::test_int(2),
                    }),
                    Value::test_record(record! {
                        "ColA" => Value::test_int(3),
                        "ColB" => Value::test_int(4),
                        "column2" => Value::test_int(5),
                    }),
                    Value::test_record(record! {
                        "ColA" => Value::test_int(6),
                    }),
                ])),
            },
            Example {
                description: "Convert comma-separated data to a table, ignoring headers.",
                example: "open data.txt | from csv --noheaders",
                result: None,
            },
            Example {
                description: "Convert semicolon-separated data to a table.",
                example: "open data.txt | from csv --separator ';'",
                result: None,
            },
            Example {
                description: "Convert comma-separated data to a table, ignoring lines starting with '#'.",
                example: "open data.txt | from csv --comment '#'",
                result: None,
            },
            Example {
                description: "Convert comma-separated data to a table, dropping all possible whitespaces around header names and field values.",
                example: "open data.txt | from csv --trim all",
                result: None,
            },
            Example {
                description: "Convert comma-separated data to a table, dropping all possible whitespaces around header names.",
                example: "open data.txt | from csv --trim headers",
                result: None,
            },
            Example {
                description: "Convert comma-separated data to a table, dropping all possible whitespaces around field values.",
                example: "open data.txt | from csv --trim fields",
                result: None,
            },
        ]
    }
}

fn from_csv(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let name = call.head;
    if let PipelineData::Value(Value::List { .. }, _) = input {
        return Err(ShellError::TypeMismatch {
            err_message: "received list stream, did you forget to open file with --raw flag?"
                .into(),
            span: name,
        });
    }

    let separator = match call.get_flag::<String>(engine_state, stack, "separator")? {
        Some(sep) => {
            if sep.len() == 1 {
                sep.chars().next().unwrap_or(',')
            } else if sep.len() == 4 {
                let unicode_sep = u32::from_str_radix(&sep, 16);
                char::from_u32(unicode_sep.unwrap_or(b'\x1f' as u32)).unwrap_or(',')
            } else {
                return Err(ShellError::NonUtf8Custom {
                    msg: "separator should be a single char or a 4-byte unicode".into(),
                    span: call.span(),
                });
            }
        }
        None => ',',
    };
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
        separator,
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
    use nu_test_support::TestResultExt;

    use super::*;

    use crate::Reject;
    use crate::{Metadata, MetadataSet};

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(FromCsv)
    }

    #[test]
    fn test_duplicate_headers_are_renamed() -> nu_test_support::Result {
        // Repeated names get a `_N` suffix so no column is shadowed.
        nu_test_support::test()
            .run(r#""name,name\nAlice,Bob" | from csv | to nuon"#)
            .expect_value_eq(r#"[[name, "name_1"]; [Alice, Bob]]"#)
    }

    #[test]
    fn test_duplicate_header_suffix_skips_existing_names() -> nu_test_support::Result {
        // A real `name_1` column keeps its name; the repeat moves past it.
        nu_test_support::test()
            .run(r#""name,name,name_1\n1,2,3" | from csv | to nuon"#)
            .expect_value_eq(r#"[[name, "name_2", "name_1"]; [1, 2, 3]]"#)
    }

    #[test]
    fn test_duplicate_headers_with_flexible() -> nu_test_support::Result {
        nu_test_support::test()
            .run(r#""a,a\n1,2,3" | from csv --flexible | to nuon"#)
            .expect_value_eq(r#"[[a, "a_1", "column2"]; [1, 2, 3]]"#)
    }

    #[test]
    fn test_blank_headers_are_named_by_position() -> nu_test_support::Result {
        // Several unlabeled header cells must not count as duplicates.
        nu_test_support::test()
            .run(r#""a,b,,\n1,2,3,4" | from csv | to nuon"#)
            .expect_value_eq(r#"[[a, b, "column2", "column3"]; [1, 2, 3, 4]]"#)
    }

    #[test]
    fn test_noheaders_keeps_repeated_columns() -> nu_test_support::Result {
        // With --noheaders the first row is data, so every column of every
        // row survives instead of being shadowed.
        nu_test_support::test()
            .run(r#""name,name\nAlice,Bob" | from csv --noheaders | to nuon"#)
            .expect_value_eq(r#"[["column0", "column1"]; [name, name], [Alice, Bob]]"#)
    }

    #[test]
    fn test_flexible_filler_names_skip_header_columns() -> nu_test_support::Result {
        // The header already contains a `column2`, so the filler name for the
        // extra field must move past it instead of repeating it.
        nu_test_support::test()
            .run(r#""a,column2\n1,2,3" | from csv --flexible | to nuon"#)
            .expect_value_eq(r#"[[a, "column2", "column3"]; [1, 2, 3]]"#)
    }

    #[test]
    fn test_content_type_metadata() {
        let mut engine_state = Box::new(EngineState::new());
        let delta = {
            let mut working_set = StateWorkingSet::new(&engine_state);

            working_set.add_decl(Box::new(FromCsv {}));
            working_set.add_decl(Box::new(Metadata {}));
            working_set.add_decl(Box::new(MetadataSet {}));
            working_set.add_decl(Box::new(Reject {}));

            working_set.render()
        };

        engine_state
            .merge_delta(delta)
            .expect("Error merging delta");

        let cmd = r#""a,b\n1,2" | metadata set --content-type 'text/csv' --path-columns [name] | from csv | metadata | reject span | $in"#;
        let result = eval_pipeline_without_terminal_expression(
            cmd,
            std::env::temp_dir().as_ref(),
            &mut engine_state,
        );
        assert_eq!(
            Value::test_record(
                record!("path_columns" => Value::test_list(vec![Value::test_string("name")]))
            ),
            result.expect("There should be a result")
        )
    }
}
