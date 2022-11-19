use super::url;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value,
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
        "Parses a url"
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
        parse(input.into_value(call.head), engine_state)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Parses a url",
            example: "'http://user123:pass567@www.example.com:8081/foo/bar?param1=section&p2=&f[name]=vldc#hello' | url parse",
            result: Some(Value::Record {
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
                    Value::Record {
                        cols: vec!["param1".to_string(), "p2".to_string(), "f[name]".to_string()],
                        vals: vec![
                            Value::test_string("section"),
                            Value::test_string(""),
                            Value::test_string("vldc"),
                        ],
                        span: Span::test_data(),
                    },
                ],
                span: Span::test_data(),
            }),
        }]
    }
}

fn get_url_string(value: &Value, engine_state: &EngineState) -> String {
    value.into_string("", engine_state.get_config())
}

fn parse(value: Value, engine_state: &EngineState) -> Result<PipelineData, ShellError> {
    let url_string = get_url_string(&value, engine_state);

    let result_url = Url::parse(url_string.as_str());

    let head = value.span()?;

    match result_url {
        Ok(url) => {
            let cols = vec![
                String::from("scheme"),
                String::from("username"),
                String::from("password"),
                String::from("host"),
                String::from("port"),
                String::from("path"),
                String::from("query"),
                String::from("fragment"),
                String::from("params"),
            ];
            let mut vals: Vec<Value> = vec![
                Value::String {
                    val: String::from(url.scheme()),
                    span: head,
                },
                Value::String {
                    val: String::from(url.username()),
                    span: head,
                },
                Value::String {
                    val: String::from(url.password().unwrap_or("")),
                    span: head,
                },
                Value::String {
                    val: String::from(url.host_str().unwrap_or("")),
                    span: head,
                },
                Value::String {
                    val: url
                        .port()
                        .map(|p| p.to_string())
                        .unwrap_or_else(|| "".into()),
                    span: head,
                },
                Value::String {
                    val: String::from(url.path()),
                    span: head,
                },
                Value::String {
                    val: String::from(url.query().unwrap_or("")),
                    span: head,
                },
                Value::String {
                    val: String::from(url.fragment().unwrap_or("")),
                    span: head,
                },
            ];

            let params =
                serde_urlencoded::from_str::<Vec<(String, String)>>(url.query().unwrap_or(""));
            match params {
                Ok(result) => {
                    let (param_cols, param_vals) = result
                        .into_iter()
                        .map(|(k, v)| (k, Value::String { val: v, span: head }))
                        .unzip();

                    vals.push(Value::Record {
                        cols: param_cols,
                        vals: param_vals,
                        span: head,
                    });

                    Ok(PipelineData::Value(
                        Value::Record {
                            cols,
                            vals,
                            span: head,
                        },
                        None,
                    ))
                }

                _ => Err(ShellError::UnsupportedInput(
                    "String not compatible with url-encoding".to_string(),
                    head,
                )),
            }
        }
        Err(_e) => Err(ShellError::UnsupportedInput(
            "Incomplete or incorrect url. Expected a full url, e.g., https://www.example.com"
                .to_string(),
            head,
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
