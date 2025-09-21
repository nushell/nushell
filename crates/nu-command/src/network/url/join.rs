use nu_engine::command_prelude::*;

use super::query::{record_to_query_string, table_to_query_string};

#[derive(Clone)]
pub struct UrlJoin;

impl Command for UrlJoin {
    fn name(&self) -> &str {
        "url join"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("url join")
            .input_output_types(vec![(Type::record(), Type::String)])
            .category(Category::Network)
    }

    fn description(&self) -> &str {
        "Converts a record to url."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec![
            "scheme", "username", "password", "hostname", "port", "path", "query", "fragment",
        ]
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Outputs a url representing the contents of this record, `params` and `query` fields must be equivalent",
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
                description: "Outputs a url representing the contents of this record, \"exploding\" the list in `params` into multiple parameters",
                example: r#"{
        "scheme": "http",
        "username": "user",
        "password": "pwd",
        "host": "www.pixiv.net",
        "port": "1234",
        "params": {a: ["one", "two"], b: "three"},
        "fragment": ""
    } | url join"#,
                result: Some(Value::test_string(
                    "http://user:pwd@www.pixiv.net:1234?a=one&a=two&b=three",
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
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;

        let output: Result<String, ShellError> = input
            .into_iter()
            .map(move |value| {
                let span = value.span();
                match value {
                    Value::Record { val, .. } => {
                        let url_components = val
                            .into_owned()
                            .into_iter()
                            .try_fold(UrlComponents::new(), |url, (k, v)| {
                                url.add_component(k, v, head, engine_state)
                            });

                        url_components?.to_url(span)
                    }
                    Value::Error { error, .. } => Err(*error),
                    other => Err(ShellError::UnsupportedInput {
                        msg: "Expected a record from pipeline".to_string(),
                        input: "value originates from here".into(),
                        msg_span: head,
                        input_span: other.span(),
                    }),
                }
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

    pub fn add_component(
        self,
        key: String,
        value: Value,
        head: Span,
        engine_state: &EngineState,
    ) -> Result<Self, ShellError> {
        let value_span = value.span();
        if key == "port" {
            return match value {
                Value::String { val, .. } => {
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
                                    "Port parameter should represent an unsigned int",
                                ),
                                span: value_span,
                            }),
                        }
                    }
                }
                Value::Int { val, .. } => Ok(Self {
                    port: Some(val),
                    ..self
                }),
                Value::Error { error, .. } => Err(*error),
                other => Err(ShellError::IncompatibleParametersSingle {
                    msg: String::from(
                        "Port parameter should be an unsigned int or a string representing it",
                    ),
                    span: other.span(),
                }),
            };
        }

        if key == "params" {
            let mut qs = match value {
                Value::Record { ref val, .. } => record_to_query_string(val, value_span, head)?,
                Value::List { ref vals, .. } => table_to_query_string(vals, value_span, head)?,
                Value::Error { error, .. } => return Err(*error),
                other => {
                    return Err(ShellError::IncompatibleParametersSingle {
                        msg: String::from("Key params has to be a record or a table"),
                        span: other.span(),
                    });
                }
            };

            qs = if !qs.trim().is_empty() {
                format!("?{qs}")
            } else {
                qs
            };

            if let Some(q) = self.query
                && q != qs
            {
                // if query is present it means that also query_span is set.
                return Err(ShellError::IncompatibleParameters {
                    left_message: format!("Mismatch, query string from params is: {qs}"),
                    left_span: value_span,
                    right_message: format!("instead query is: {q}"),
                    right_span: self.query_span.unwrap_or(Span::unknown()),
                });
            }

            return Ok(Self {
                query: Some(qs),
                params_span: Some(value_span),
                ..self
            });
        }

        // apart from port and params all other keys are strings.
        let s = value.coerce_into_string()?; // If value fails String conversion, just output this ShellError
        if !Self::check_empty_string_ok(&key, &s, value_span)? {
            return Ok(self);
        }
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
                if let Some(q) = self.query
                    && q != s
                {
                    // if query is present it means that also params_span is set.
                    return Err(ShellError::IncompatibleParameters {
                        left_message: format!("Mismatch, query param is: {s}"),
                        left_span: value_span,
                        right_message: format!("instead query string from params is: {q}"),
                        right_span: self.params_span.unwrap_or(Span::unknown()),
                    });
                }

                Ok(Self {
                    query: Some(format!("?{s}")),
                    query_span: Some(value_span),
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
            _ => {
                nu_protocol::report_shell_error(
                    engine_state,
                    &ShellError::GenericError {
                        error: format!("'{key}' is not a valid URL field"),
                        msg: format!("remove '{key}' col from input record"),
                        span: Some(value_span),
                        help: None,
                        inner: vec![],
                    },
                );
                Ok(self)
            }
        }
    }

    // Check if value is empty. If so, check if that is fine, i.e., not a required input
    fn check_empty_string_ok(key: &str, s: &str, value_span: Span) -> Result<bool, ShellError> {
        if !s.trim().is_empty() {
            return Ok(true);
        }
        match key {
            "host" => Err(ShellError::InvalidValue {
                valid: "a non-empty string".into(),
                actual: format!("'{s}'"),
                span: value_span,
            }),
            "scheme" => Err(ShellError::InvalidValue {
                valid: "a non-empty string".into(),
                actual: format!("'{s}'"),
                span: value_span,
            }),
            _ => Ok(false),
        }
    }

    pub fn to_url(&self, span: Span) -> Result<String, ShellError> {
        let user_and_pwd = match (&self.username, &self.password) {
            (Some(usr), Some(pwd)) => format!("{usr}:{pwd}@"),
            (Some(usr), None) => format!("{usr}@"),
            _ => String::from(""),
        };

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

        test_examples(UrlJoin {})
    }
}
