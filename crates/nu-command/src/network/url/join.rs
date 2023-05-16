use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, Type, Value,
};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "url join"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("url join")
            .input_output_types(vec![(Type::Record(vec![]), Type::String)])
            .category(Category::Network)
    }

    fn usage(&self) -> &str {
        "Converts a record to url."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec![
            "scheme", "username", "password", "hostname", "port", "path", "query", "fragment",
        ]
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Outputs a url representing the contents of this record",
                example: r#"{
        "scheme": "http",
        "username": "",
        "password": "",
        "host": "www.pixiv.net",
        "port": "",
        "path": "/member_illust.php",
        "query": "mode=medium&illust_id=99260204",
        "fragment": "",
        "params":
        {
            "mode": "medium",
            "illust_id": "99260204"
        }
    } | url join"#,
                result: Some(Value::test_string(
                    "http://www.pixiv.net/member_illust.php?mode=medium&illust_id=99260204",
                )),
            },
            Example {
                description: "Outputs a url representing the contents of this record",
                example: r#"{
        "scheme": "http",
        "username": "user",
        "password": "pwd",
        "host": "www.pixiv.net",
        "port": "1234",
        "query": "test=a",
        "fragment": ""
    } | url join"#,
                result: Some(Value::test_string(
                    "http://user:pwd@www.pixiv.net:1234?test=a",
                )),
            },
            Example {
                description: "Outputs a url representing the contents of this record",
                example: r#"{
        "scheme": "http",
        "host": "www.pixiv.net",
        "port": "1234",
        "path": "user",
        "fragment": "frag"
    } | url join"#,
                result: Some(Value::test_string("http://www.pixiv.net:1234/user#frag")),
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

        let output: Result<String, ShellError> = input
            .into_iter()
            .map(move |value| match value {
                Value::Record {
                    ref cols,
                    ref vals,
                    span,
                } => {
                    let url_components = cols
                        .iter()
                        .zip(vals.iter())
                        .fold(Ok(UrlComponents::new()), |url, (k, v)| {
                            url?.add_component(k.clone(), v.clone(), span)
                        });

                    url_components?.to_url(span)
                }
                Value::Error { error } => Err(*error),
                other => Err(ShellError::UnsupportedInput(
                    "Expected a record from pipeline".to_string(),
                    "value originates from here".into(),
                    head,
                    other.expect_span(),
                )),
            })
            .collect();

        Ok(Value::string(output?, head).into_pipeline_data())
    }
}

#[derive(Default)]
struct UrlComponents {
    scheme: Option<String>,
    username: Option<String>,
    password: Option<String>,
    host: Option<String>,
    port: Option<i64>,
    path: Option<String>,
    query: Option<String>,
    fragment: Option<String>,
    query_span: Option<Span>,
    params_span: Option<Span>,
}

impl UrlComponents {
    fn new() -> Self {
        Default::default()
    }

    pub fn add_component(self, key: String, value: Value, _span: Span) -> Result<Self, ShellError> {
        if key == "port" {
            return match value {
                Value::String { val, span } => {
                    if val.trim().is_empty() {
                        Ok(self)
                    } else {
                        match val.parse::<i64>() {
                            Ok(p) => Ok(Self {
                                port: Some(p),
                                ..self
                            }),
                            Err(_) => Err(ShellError::IncompatibleParametersSingle {
                                msg: String::from(
                                    "Port parameter should represent an unsigned integer",
                                ),
                                span,
                            }),
                        }
                    }
                }
                Value::Int { val, span: _ } => Ok(Self {
                    port: Some(val),
                    ..self
                }),
                Value::Error { error } => Err(*error),
                other => Err(ShellError::IncompatibleParametersSingle {
                    msg: String::from(
                        "Port parameter should be an unsigned integer or a string representing it",
                    ),
                    span: other.expect_span(),
                }),
            };
        }

        if key == "params" {
            return match value {
                Value::Record {
                    ref cols,
                    ref vals,
                    span,
                } => {
                    let mut qs = cols
                        .iter()
                        .zip(vals.iter())
                        .map(|(k, v)| match v.as_string() {
                            Ok(val) => Ok(format!("{k}={val}")),
                            Err(err) => Err(err),
                        })
                        .collect::<Result<Vec<String>, ShellError>>()?
                        .join("&");

                    qs = format!("?{qs}");

                    if let Some(q) = self.query {
                        if q != qs {
                            // if query is present it means that also query_span is set.
                            return Err(ShellError::IncompatibleParameters {
                                left_message: format!("Mismatch, qs from params is: {qs}"),
                                left_span: value.expect_span(),
                                right_message: format!("instead query is: {q}"),
                                right_span: self.query_span.unwrap_or(Span::unknown()),
                            });
                        }
                    }

                    Ok(Self {
                        query: Some(qs),
                        params_span: Some(span),
                        ..self
                    })
                }
                Value::Error { error } => Err(*error),
                other => Err(ShellError::IncompatibleParametersSingle {
                    msg: String::from("Key params has to be a record"),
                    span: other.expect_span(),
                }),
            };
        }

        // a part from port and params all other keys are strings.
        match value.as_string() {
            Ok(s) => {
                if s.trim().is_empty() {
                    Ok(self)
                } else {
                    match key.as_str() {
                        "host" => Ok(Self {
                            host: Some(s),
                            ..self
                        }),
                        "scheme" => Ok(Self {
                            scheme: Some(s),
                            ..self
                        }),
                        "username" => Ok(Self {
                            username: Some(s),
                            ..self
                        }),
                        "password" => Ok(Self {
                            password: Some(s),
                            ..self
                        }),
                        "path" => Ok(Self {
                            path: Some(if s.starts_with('/') {
                                s
                            } else {
                                format!("/{s}")
                            }),
                            ..self
                        }),
                        "query" => {
                            if let Some(q) = self.query {
                                if q != s {
                                    // if query is present it means that also params_span is set.
                                    return Err(ShellError::IncompatibleParameters {
                                        left_message: format!("Mismatch, query param is: {s}"),
                                        left_span: value.expect_span(),
                                        right_message: format!("instead qs from params is: {q}"),
                                        right_span: self.params_span.unwrap_or(Span::unknown()),
                                    });
                                }
                            }

                            Ok(Self {
                                query: Some(format!("?{s}")),
                                query_span: Some(value.expect_span()),
                                ..self
                            })
                        }
                        "fragment" => Ok(Self {
                            fragment: Some(if s.starts_with('#') {
                                s
                            } else {
                                format!("#{s}")
                            }),
                            ..self
                        }),
                        _ => Ok(self),
                    }
                }
            }
            _ => Ok(self),
        }
    }

    pub fn to_url(&self, span: Span) -> Result<String, ShellError> {
        let mut user_and_pwd: String = String::from("");

        if let Some(usr) = &self.username {
            if let Some(pwd) = &self.password {
                user_and_pwd = format!("{usr}:{pwd}@");
            }
        }

        let scheme_result = match &self.scheme {
            Some(s) => Ok(s),
            None => Err(UrlComponents::generate_shell_error_for_missing_parameter(
                String::from("scheme"),
                span,
            )),
        };

        let host_result = match &self.host {
            Some(h) => Ok(h),
            None => Err(UrlComponents::generate_shell_error_for_missing_parameter(
                String::from("host"),
                span,
            )),
        };

        Ok(format!(
            "{}://{}{}{}{}{}{}",
            scheme_result?,
            user_and_pwd,
            host_result?,
            self.port
                .map(|p| format!(":{p}"))
                .as_deref()
                .unwrap_or_default(),
            self.path.as_deref().unwrap_or_default(),
            self.query.as_deref().unwrap_or_default(),
            self.fragment.as_deref().unwrap_or_default()
        ))
    }

    fn generate_shell_error_for_missing_parameter(pname: String, span: Span) -> ShellError {
        ShellError::MissingParameter {
            param_name: pname,
            span,
        }
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
