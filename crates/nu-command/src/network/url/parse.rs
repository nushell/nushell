use nu_engine::command_prelude::*;
use url::Url;

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
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

    fn usage(&self) -> &str {
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
        _: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        parse(input.into_value(call.head), call.head, engine_state)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Parses a url",
            example: "'http://user123:pass567@www.example.com:8081/foo/bar?param1=section&p2=&f[name]=vldc#hello' | url parse",
            result: Some(Value::test_record(record! {
                    "scheme" =>   Value::test_string("http"),
                    "username" => Value::test_string("user123"),
                    "password" => Value::test_string("pass567"),
                    "host" =>     Value::test_string("www.example.com"),
                    "port" =>     Value::test_string("8081"),
                    "path" =>     Value::test_string("/foo/bar"),
                    "query" =>    Value::test_string("param1=section&p2=&f[name]=vldc"),
                    "fragment" => Value::test_string("hello"),
                    "params" =>   Value::test_record(record! {
                        "param1" =>  Value::test_string("section"),
                        "p2" =>      Value::test_string(""),
                        "f[name]" => Value::test_string("vldc"),
                    }),
            })),
        }]
    }
}

fn get_url_string(value: &Value, engine_state: &EngineState) -> String {
    value.to_expanded_string("", engine_state.get_config())
}

fn parse(value: Value, head: Span, engine_state: &EngineState) -> Result<PipelineData, ShellError> {
    let url_string = get_url_string(&value, engine_state);

    let result_url = Url::parse(url_string.as_str());

    // This is the span of the original string, not the call head.
    let span = value.span();

    match result_url {
        Ok(url) => {
            let params =
                serde_urlencoded::from_str::<Vec<(String, String)>>(url.query().unwrap_or(""));
            match params {
                Ok(result) => {
                    let params = result
                        .into_iter()
                        .map(|(k, v)| (k, Value::string(v, head)))
                        .collect();

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
                        "params" => Value::record(params, head),
                    };

                    Ok(PipelineData::Value(Value::record(record, head), None))
                }
                _ => Err(ShellError::UnsupportedInput {
                    msg: "String not compatible with url-encoding".to_string(),
                    input: "value originates from here".into(),
                    msg_span: head,
                    input_span: span,
                }),
            }
        }
        Err(_e) => Err(ShellError::UnsupportedInput {
            msg: "Incomplete or incorrect URL. Expected a full URL, e.g., https://www.example.com"
                .to_string(),
            input: "value originates from here".into(),
            msg_span: head,
            input_span: span,
        }),
    }
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
