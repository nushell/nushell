use nu_engine::command_prelude::*;
use nu_protocol::Config;
use url::Url;

use super::query::query_string_to_table;

#[derive(Clone)]
pub struct UrlParse;

impl Command for UrlParse {
    fn name(&self) -> &str {
        "url parse"
    }

    fn signature(&self) -> Signature {
        Signature::build("url parse")
            .input_output_types(vec![
                (Type::String, Type::record()),
                (Type::table(), Type::table()),
                (Type::record(), Type::record()),
            ])
            .allow_variants_without_examples(true)
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "Optionally operate by cell path.",
            )
            .category(Category::Network)
    }

    fn description(&self) -> &str {
        "Parses a url."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec![
            "scheme", "username", "password", "hostname", "port", "path", "query", "fragment",
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        parse(
            input.into_value(call.head)?,
            call.head,
            &stack.get_config(engine_state),
        )
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Parses a url",
            example: "'http://user123:pass567@www.example.com:8081/foo/bar?param1=section&p2=&f[name]=vldc&f[no]=42#hello' | url parse",
            result: Some(Value::test_record(record! {
                    "scheme" =>   Value::test_string("http"),
                    "username" => Value::test_string("user123"),
                    "password" => Value::test_string("pass567"),
                    "host" =>     Value::test_string("www.example.com"),
                    "port" =>     Value::test_string("8081"),
                    "path" =>     Value::test_string("/foo/bar"),
                    "query" =>    Value::test_string("param1=section&p2=&f[name]=vldc&f[no]=42"),
                    "fragment" => Value::test_string("hello"),
                    "params" =>   Value::test_list(vec![
                        Value::test_record(record! {"key" => Value::test_string("param1"), "value" => Value::test_string("section") }),
                        Value::test_record(record! {"key" => Value::test_string("p2"), "value" => Value::test_string("") }),
                        Value::test_record(record! {"key" => Value::test_string("f[name]"), "value" => Value::test_string("vldc") }),
                        Value::test_record(record! {"key" => Value::test_string("f[no]"), "value" => Value::test_string("42") }),
                    ]),
            })),
        }]
    }
}

fn get_url_string(value: &Value, config: &Config) -> String {
    value.to_expanded_string("", config)
}

fn parse(value: Value, head: Span, config: &Config) -> Result<PipelineData, ShellError> {
    let url_string = get_url_string(&value, config);

    // This is the span of the original string, not the call head.
    let span = value.span();

    let url = Url::parse(url_string.as_str()).map_err(|_| ShellError::UnsupportedInput {
        msg: "Incomplete or incorrect URL. Expected a full URL, e.g., https://www.example.com"
            .to_string(),
        input: "value originates from here".into(),
        msg_span: head,
        input_span: span,
    })?;

    let params = query_string_to_table(url.query().unwrap_or(""), head, span).map_err(|_| {
        ShellError::UnsupportedInput {
            msg: "String not compatible with url-encoding".to_string(),
            input: "value originates from here".into(),
            msg_span: head,
            input_span: span,
        }
    })?;

    let port = url.port().map(|p| p.to_string()).unwrap_or_default();

    let record = record! {
        "scheme" => Value::string(url.scheme(), head),
        "username" => Value::string(url.username(), head),
        "password" => Value::string(url.password().unwrap_or(""), head),
        "host" => Value::string(url.host_str().unwrap_or(""), head),
        "port" => Value::string(port, head),
        "path" => Value::string(url.path(), head),
        "query" => Value::string(url.query().unwrap_or(""), head),
        "fragment" => Value::string(url.fragment().unwrap_or(""), head),
        "params" => params,
    };

    Ok(PipelineData::value(Value::record(record, head), None))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(UrlParse {})
    }
}
