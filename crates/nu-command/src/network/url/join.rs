use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, IntoPipelineData, ShellError, Signature, Span, Type, Value};

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
        "Converts a record to url"
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &nu_protocol::ast::Call,
        input: nu_protocol::PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
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

                    url_components?.to_url(head, span)
                }
                Value::Error { error } => Err(error),
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

#[derive(Default, Debug)]
struct UrlComponents {
    scheme: Option<String>,
    username: Option<String>,
    password: Option<String>,
    host: Option<String>,
    port: Option<i64>,
    path: Option<String>,
    query: Option<String>,
    fragment: Option<String>,
}

impl UrlComponents {
    fn new() -> Self {
        Default::default()
    }

    pub fn add_component(
        self: Self,
        key: String,
        value: Value,
        span: Span,
    ) -> Result<Self, ShellError> {
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
                            Err(_) => Err(ShellError::IncompatibleParametersSingle(
                                String::from("Port parameter should represent an unsigned integer"),
                                span,
                            )),
                        }
                    }
                }
                Value::Int { val, span } => Ok(Self {
                    port: Some(val),
                    ..self
                }),
                other => Err(ShellError::IncompatibleParametersSingle(
                    String::from(
                        "Port parameter should be an unsigned integer or a string representing it",
                    ),
                    other.expect_span(),
                )),
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
                            Ok(val) => Ok(format!("{}={}", k, val)),
                            Err(err) => Err(err),
                        })
                        .collect::<Result<Vec<String>, ShellError>>()?
                        .join("&")
                        .to_string();

                    qs = format!("?{}", qs);

                    if let Some(q) = self.query {
                        if q != qs {
                            return Err(ShellError::IncompatibleParametersSingle(
                                String::from("Parameters query and params are in conflict"),
                                span,
                            ));
                        }
                    }

                    Ok(Self {
                        query: Some(qs),
                        ..self
                    })
                }
                Value::Error { error } => Err(error),
                other => Err(ShellError::IncompatibleParametersSingle(
                    String::from("Key params has to be a record"),
                    other.expect_span(),
                )),
            };
        }

        match value.as_string() {
            Ok(s) => match key.as_str() {
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
                        format!("/{}", s)
                    }),
                    ..self
                }),
                "query" => {
                    if let Some(q) = self.query {
                        if q != s {
                            return Err(ShellError::IncompatibleParametersSingle(
                                String::from("Parameters query and params are in conflict"),
                                span,
                            ));
                        }
                    }

                    Ok(Self {
                        query: Some(format!("?{}", s)),
                        ..self
                    })
                }
                "fragment" => Ok(Self {
                    fragment: Some(if s.starts_with("#") {
                        s
                    } else {
                        format!("#{}", s)
                    }),
                    ..self
                }),
                _ => Ok(self),
            },
            _ => Ok(self),
        }
    }

    pub fn to_url(self: &Self, head: Span, span: Span) -> Result<String, ShellError> {
        let mut user_and_pwd: String = String::from("");

        if let Some(usr) = &self.username {
            if let Some(pwd) = &self.password {
                user_and_pwd = format!("{}:{}@", usr, pwd);
            }
        }

        Ok(format!(
            "{}://{}{}{}{}{}{}",
            self.scheme.as_ref().ok_or(
                UrlComponents::generate_shell_error_for_missing_parameter("scheme", head, span)
            )?,
            user_and_pwd,
            self.host
                .as_ref()
                .ok_or(UrlComponents::generate_shell_error_for_missing_parameter(
                    "host", head, span
                ))?,
            self.port
                .map(|p| format!(":{}", p))
                .as_ref()
                .unwrap_or(&String::from("")),
            self.path.as_ref().unwrap_or(&String::from("")),
            self.query.as_ref().unwrap_or(&String::from("")),
            self.fragment.as_ref().unwrap_or(&String::from(""))
        ))
    }

    fn generate_shell_error_for_missing_parameter(
        pname: &str,
        head: Span,
        span: Span,
    ) -> ShellError {
        ShellError::UnsupportedInput(
            format!("Missing required param: {}", pname),
            String::from("value originates from here"),
            head,
            span,
        )
    }
}
