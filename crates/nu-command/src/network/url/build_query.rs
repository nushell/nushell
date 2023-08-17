use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, SpannedValue,
    Type,
};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "url build-query"
    }

    fn signature(&self) -> Signature {
        Signature::build("url build-query")
            .input_output_types(vec![
                (Type::Record(vec![]), Type::String),
                (Type::Table(vec![]), Type::String),
            ])
            .category(Category::Network)
    }

    fn usage(&self) -> &str {
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
                result: Some(SpannedValue::test_string("mode=normal&userid=31415")),
            },
            Example {
                description: "Outputs a query string representing the contents of this 1-row table",
                example: r#"[[foo bar]; ["1" "2"]] | url build-query"#,
                result: Some(SpannedValue::test_string("foo=1&bar=2")),
            },
            Example {
                description: "Outputs a query string representing the contents of this record",
                example: r#"{a:"AT&T", b: "AT T"} | url build-query"#,
                result: Some(SpannedValue::test_string("a=AT%26T&b=AT+T")),
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
        .map(move |value| match value {
            SpannedValue::Record {
                ref cols,
                ref vals,
                span,
            } => {
                let mut row_vec = vec![];
                for (k, v) in cols.iter().zip(vals.iter()) {
                    match v.as_string() {
                        Ok(s) => {
                            row_vec.push((k.clone(), s.to_string()));
                        }
                        _ => {
                            return Err(ShellError::UnsupportedInput(
                                "Expected a record with string values".to_string(),
                                "value originates from here".into(),
                                head,
                                span,
                            ));
                        }
                    }
                }

                match serde_urlencoded::to_string(row_vec) {
                    Ok(s) => Ok(s),
                    _ => Err(ShellError::CantConvert {
                        to_type: "URL".into(),
                        from_type: value.get_type().to_string(),
                        span: head,
                        help: None,
                    }),
                }
            }
            // Propagate existing errors
            SpannedValue::Error { error } => Err(*error),
            other => Err(ShellError::UnsupportedInput(
                "Expected a table from pipeline".to_string(),
                "value originates from here".into(),
                head,
                other.expect_span(),
            )),
        })
        .collect();

    Ok(SpannedValue::string(output?, head).into_pipeline_data())
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
