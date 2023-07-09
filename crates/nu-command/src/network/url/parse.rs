use super::url;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    record, Category, Example, PipelineData, Record, ShellError, Signature, Span, SyntaxShape,
    Type, Value,
};

use url::Url;

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "url parse"
    }

    fn signature(&self) -> Signature {
        Signature::build("url parse")
            .input_output_types(vec![(Type::String, Type::Record(vec![]))])
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "optionally operate by cell path",
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
            result: Some(Value::test_record(Record {
                cols: vec![
                    "scheme".to_string(),
                    "username".to_string(),
                    "password".to_string(),
                    "host".to_string(),
                    "port".to_string(),
                    "path".to_string(),
                    "query".to_string(),
                    "fragment".to_string(),
                    "params".to_string(),
                ],
                vals: vec![
                    Value::test_string("http"),
                    Value::test_string("user123"),
                    Value::test_string("pass567"),
                    Value::test_string("www.example.com"),
                    Value::test_string("8081"),
                    Value::test_string("/foo/bar"),
                    Value::test_string("param1=section&p2=&f[name]=vldc"),
                    Value::test_string("hello"),
                    Value::test_record(Record {
                        cols: vec!["param1".to_string(), "p2".to_string(), "f[name]".to_string()],
                        vals: vec![
                            Value::test_string("section"),
                            Value::test_string(""),
                            Value::test_string("vldc"),
                        ],
                    }),
                ],
            })),
        }]
    }
}

fn get_url_string(value: &Value, engine_state: &EngineState) -> String {
    value.into_string("", engine_state.get_config())
}

fn parse(value: Value, head: Span, engine_state: &EngineState) -> Result<PipelineData, ShellError> {
    let url_string = get_url_string(&value, engine_state);

    let result_url = Url::parse(url_string.as_str());

    // This is the span of the original string, not the call head.
    let span = value.span()?;

    match result_url {
        Ok(url) => {
            let params = match serde_urlencoded::from_str::<Vec<(String, String)>>(
                url.query().unwrap_or(""),
            ) {
                Ok(result) => result
                    .into_iter()
                    .map(|(k, v)| (k, Value::string(v, head)))
                    .collect(),

                _ => {
                    return Err(ShellError::UnsupportedInput(
                        "String not compatible with url-encoding".to_string(),
                        "value originates from here".into(),
                        head,
                        span,
                    ))
                }
            };

            let record = record! {
                scheme => Value::string(url.scheme(), head),
                username => Value::string(url.username(), head),
                password => Value::string(url.password().unwrap_or(""), head),
                host => Value::string(url.host_str().unwrap_or(""), head),
                port => Value::string(url.port().map(|p| p.to_string()).unwrap_or_default(), head),
                path => Value::string(url.path(), head),
                query => Value::string(url.query().unwrap_or(""), head),
                fragment => Value::string(url.fragment().unwrap_or(""), head),
                params => Value::record(params, head),
            };

            Ok(PipelineData::Value(Value::record(record, span), None))
        }
        Err(_e) => Err(ShellError::UnsupportedInput(
            "Incomplete or incorrect URL. Expected a full URL, e.g., https://www.example.com"
                .to_string(),
            "value originates from here".into(),
            head,
            span,
        )),
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
