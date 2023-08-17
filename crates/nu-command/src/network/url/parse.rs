use super::url;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Span, SpannedValue, SyntaxShape, Type,
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
            .input_output_types(vec![
                (Type::String, Type::Record(vec![])),
                (Type::Table(vec![]), Type::Table(vec![])),
                (Type::Record(vec![]), Type::Record(vec![])),
            ])
            .allow_variants_without_examples(true)
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
            result: Some(SpannedValue::Record {
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
                    SpannedValue::test_string("http"),
                    SpannedValue::test_string("user123"),
                    SpannedValue::test_string("pass567"),
                    SpannedValue::test_string("www.example.com"),
                    SpannedValue::test_string("8081"),
                    SpannedValue::test_string("/foo/bar"),
                    SpannedValue::test_string("param1=section&p2=&f[name]=vldc"),
                    SpannedValue::test_string("hello"),
                    SpannedValue::Record {
                        cols: vec!["param1".to_string(), "p2".to_string(), "f[name]".to_string()],
                        vals: vec![
                            SpannedValue::test_string("section"),
                            SpannedValue::test_string(""),
                            SpannedValue::test_string("vldc"),
                        ],
                        span: Span::test_data(),
                    },
                ],
                span: Span::test_data(),
            }),
        }]
    }
}

fn get_url_string(value: &SpannedValue, engine_state: &EngineState) -> String {
    value.into_string("", engine_state.get_config())
}

fn parse(
    value: SpannedValue,
    head: Span,
    engine_state: &EngineState,
) -> Result<PipelineData, ShellError> {
    let url_string = get_url_string(&value, engine_state);

    let result_url = Url::parse(url_string.as_str());

    // This is the span of the original string, not the call head.
    let span = value.span();

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
            let mut vals: Vec<SpannedValue> = vec![
                SpannedValue::String {
                    val: String::from(url.scheme()),
                    span: head,
                },
                SpannedValue::String {
                    val: String::from(url.username()),
                    span: head,
                },
                SpannedValue::String {
                    val: String::from(url.password().unwrap_or("")),
                    span: head,
                },
                SpannedValue::String {
                    val: String::from(url.host_str().unwrap_or("")),
                    span: head,
                },
                SpannedValue::String {
                    val: url
                        .port()
                        .map(|p| p.to_string())
                        .unwrap_or_else(|| "".into()),
                    span: head,
                },
                SpannedValue::String {
                    val: String::from(url.path()),
                    span: head,
                },
                SpannedValue::String {
                    val: String::from(url.query().unwrap_or("")),
                    span: head,
                },
                SpannedValue::String {
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
                        .map(|(k, v)| (k, SpannedValue::String { val: v, span: head }))
                        .unzip();

                    vals.push(SpannedValue::Record {
                        cols: param_cols,
                        vals: param_vals,
                        span: head,
                    });

                    Ok(PipelineData::Value(
                        SpannedValue::Record {
                            cols,
                            vals,
                            span: head,
                        },
                        None,
                    ))
                }

                _ => Err(ShellError::UnsupportedInput(
                    "String not compatible with url-encoding".to_string(),
                    "value originates from here".into(),
                    head,
                    span,
                )),
            }
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
