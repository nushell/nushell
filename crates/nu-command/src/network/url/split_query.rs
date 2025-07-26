use nu_engine::command_prelude::*;

use super::query::query_string_to_table;

#[derive(Clone)]
pub struct UrlSplitQuery;

impl Command for UrlSplitQuery {
    fn name(&self) -> &str {
        "url split-query"
    }

    fn signature(&self) -> Signature {
        Signature::build("url split-query")
            .input_output_types(vec![(
                Type::String,
                Type::Table([("key".into(), Type::String), ("value".into(), Type::String)].into()),
            )])
            .category(Category::Network)
    }

    fn description(&self) -> &str {
        "Converts query string into table applying percent-decoding."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["convert", "record", "table"]
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Outputs a table representing the contents of this query string",
                example: r#""mode=normal&userid=31415" | url split-query"#,
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "key" => Value::test_string("mode"),
                        "value" => Value::test_string("normal"),
                    }),
                    Value::test_record(record! {
                        "key" => Value::test_string("userid"),
                        "value" => Value::test_string("31415"),
                    }),
                ])),
            },
            Example {
                description: "Outputs a table representing the contents of this query string, url-decoding the values",
                example: r#""a=AT%26T&b=AT+T" | url split-query"#,
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "key" => Value::test_string("a"),
                        "value" => Value::test_string("AT&T"),
                    }),
                    Value::test_record(record! {
                        "key" => Value::test_string("b"),
                        "value" => Value::test_string("AT T"),
                    }),
                ])),
            },
            Example {
                description: "Outputs a table representing the contents of this query string",
                example: r#""a=one&a=two&b=three" | url split-query"#,
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "key" => Value::test_string("a"),
                        "value" => Value::test_string("one"),
                    }),
                    Value::test_record(record! {
                        "key" => Value::test_string("a"),
                        "value" => Value::test_string("two"),
                    }),
                    Value::test_record(record! {
                        "key" => Value::test_string("b"),
                        "value" => Value::test_string("three"),
                    }),
                ])),
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
        let value = input.into_value(call.head)?;
        let span = value.span();
        let query = value.to_expanded_string("", &stack.get_config(engine_state));
        let table = query_string_to_table(&query, call.head, span)?;
        Ok(PipelineData::value(table, None))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(UrlSplitQuery {})
    }
}
