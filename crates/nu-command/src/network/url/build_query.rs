use nu_engine::command_prelude::*;

use super::query::record_to_query_string;

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "url build-query"
    }

    fn signature(&self) -> Signature {
        Signature::build("url build-query")
            .input_output_types(vec![
                (Type::record(), Type::String),
                (Type::table(), Type::String),
            ])
            .category(Category::Network)
    }

    fn description(&self) -> &str {
        "Converts record or table into query string applying percent-encoding."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["convert", "record", "table"]
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Outputs a query string representing the contents of this record",
                example: r#"{ mode:normal userid:31415 } | url build-query"#,
                result: Some(Value::test_string("mode=normal&userid=31415")),
            },
            Example {
                description: "Outputs a query string representing the contents of this 1-row table",
                example: r#"[[foo bar]; ["1" "2"]] | url build-query"#,
                result: Some(Value::test_string("foo=1&bar=2")),
            },
            Example {
                description: "Outputs a query string representing the contents of this record, with a value that needs to be url-encoded",
                example: r#"{a:"AT&T", b: "AT T"} | url build-query"#,
                result: Some(Value::test_string("a=AT%26T&b=AT+T")),
            },
            Example {
                description: "Outputs a query string representing the contents of this record, \"exploding\" the list into multiple parameters",
                example: r#"{a: ["one", "two"], b: "three"} | url build-query"#,
                result: Some(Value::test_string("a=one&a=two&b=three")),
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
        to_url(input, head)
    }
}

fn to_url(input: PipelineData, head: Span) -> Result<PipelineData, ShellError> {
    let output: Result<String, ShellError> = input
        .into_iter()
        .map(move |value| {
            let span = value.span();
            match value {
                Value::Record { ref val, .. } => record_to_query_string(val, span, head),
                // Propagate existing errors
                Value::Error { error, .. } => Err(*error),
                other => Err(ShellError::UnsupportedInput {
                    msg: "Expected a table from pipeline".to_string(),
                    input: "value originates from here".into(),
                    msg_span: head,
                    input_span: other.span(),
                }),
            }
        })
        .collect();

    Ok(Value::string(output?, head).into_pipeline_data())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(SubCommand {})
    }
}
