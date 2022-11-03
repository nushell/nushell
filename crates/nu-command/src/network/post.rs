use crate::formats::value_to_json_value;
use crate::BufferedReader;
use base64::encode;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::RawStream;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use ureq::{AgentBuilder, Error, Response};

use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Value,
};
use std::collections::HashMap;
use std::io::BufReader;

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "post"
    }

    fn signature(&self) -> Signature {
        Signature::build("post")
            .required("path", SyntaxShape::String, "the URL to post to")
            .required("body", SyntaxShape::Any, "the contents of the post body")
            .named(
                "user",
                SyntaxShape::Any,
                "the username when authenticating",
                Some('u'),
            )
            .named(
                "password",
                SyntaxShape::Any,
                "the password when authenticating",
                Some('p'),
            )
            .named(
                "content-type",
                SyntaxShape::Any,
                "the MIME type of content to post",
                Some('t'),
            )
            .named(
                "content-length",
                SyntaxShape::Any,
                "the length of the content being posted",
                Some('l'),
            )
            .named(
                "headers",
                SyntaxShape::Any,
                "custom headers you want to add ",
                Some('H'),
            )
            .switch(
                "raw",
                "return values as a string instead of a table",
                Some('r'),
            )
            .filter()
            .category(Category::Network)
    }

    fn usage(&self) -> &str {
        "Post a body to a URL."
    }

    fn extra_usage(&self) -> &str {
        "Performs HTTP POST operation."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["network", "send", "push", "http"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        run_post(engine_state, stack, call, input)
    }
    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Post content to url.com",
                example: "post url.com 'body'",
                result: None,
            },
            Example {
                description: "Post content to url.com, with username and password",
                example: "post -u myuser -p mypass url.com 'body'",
                result: None,
            },
            Example {
                description: "Post content to url.com, with custom header",
                example: "post -H [my-header-key my-header-value] url.com",
                result: None,
            },
            Example {
                description: "Post content to url.com with a json body",
                example: "post -t application/json url.com { field: value }",
                result: None,
            },
        ]
    }
}

struct Arguments {
    path: Option<Value>,
    body: Option<Value>,
    headers: Option<Value>,
    raw: bool,
    user: Option<String>,
    password: Option<String>,
    content_type: Option<String>,
    content_length: Option<String>,
}

#[derive(PartialEq, Eq)]
enum BodyType {
    Json,
    Form,
    Unknown,
}

fn run_post(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
    let args = Arguments {
        path: Some(call.req(engine_state, stack, 0)?),
        body: Some(call.req(engine_state, stack, 1)?),
        headers: call.get_flag(engine_state, stack, "headers")?,
        raw: call.has_flag("raw"),
        user: call.get_flag(engine_state, stack, "user")?,
        password: call.get_flag(engine_state, stack, "password")?,
        content_type: call.get_flag(engine_state, stack, "content-type")?,
        content_length: call.get_flag(engine_state, stack, "content-length")?,
    };
    helper(engine_state, stack, call, args)
}
// Helper function that actually goes to retrieve the resource from the url given
// The Option<String> return a possible file extension which can be used in AutoConvert commands
fn helper(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    args: Arguments,
) -> std::result::Result<PipelineData, ShellError> {
    let url_value = if let Some(val) = args.path {
        val
    } else {
        return Err(ShellError::UnsupportedInput(
            "Expecting a URL as a string but got nothing".to_string(),
            call.head,
        ));
    };
    let body = if let Some(body) = args.body {
        body
    } else {
        return Err(ShellError::UnsupportedInput(
            "Expecting a body parameter but got nothing".to_string(),
            call.head,
        ));
    };
    let span = url_value.span()?;
    let requested_url = url_value.as_string()?;
    let url = match url::Url::parse(&requested_url) {
        Ok(u) => u,
        Err(_e) => {
            return Err(ShellError::UnsupportedInput(
                "Incomplete or incorrect URL. Expected a full URL, e.g., https://www.example.com"
                    .to_string(),
                span,
            ));
        }
    };
    let user = args.user.clone();
    let password = args.password;
    let headers = args.headers;
    let location = url;
    let raw = args.raw;
    let login = match (user, password) {
        (Some(user), Some(password)) => Some(encode(&format!("{}:{}", user, password))),
        (Some(user), _) => Some(encode(&format!("{}:", user))),
        _ => None,
    };

    let body_type = match &args.content_type {
        Some(it) if it == "application/json" => BodyType::Json,
        Some(it) if it == "application/x-www-form-urlencoded" => BodyType::Form,
        _ => BodyType::Unknown,
    };

    let mut request = AgentBuilder::new()
        .tls_connector(Arc::new(native_tls::TlsConnector::new().map_err(|e| {
            ShellError::GenericError(
                "Failed to load tls.".to_string(),
                e.to_string(),
                None,
                None,
                Vec::new(),
            )
        })?))
        .build()
        .post(location.as_str())
        .set("User-Agent", "nushell");

    // set the content-type header before using e.g., request.json
    // because that will avoid duplicating the header value
    if let Some(val) = args.content_type {
        request = request.set("Content-Type", &val);
    }

    if let Some(login) = login {
        request = request.set("Authorization", &format!("Basic {}", login));
    }

    if let Some(val) = args.content_length {
        request = request.set("Content-Length", &val);
    }

    if let Some(headers) = headers {
        let mut custom_headers: HashMap<String, Value> = HashMap::new();

        match &headers {
            Value::List { vals: table, .. } => {
                if table.len() == 1 {
                    // single row([key1 key2]; [val1 val2])
                    match &table[0] {
                        Value::Record { cols, vals, .. } => {
                            for (k, v) in cols.iter().zip(vals.iter()) {
                                custom_headers.insert(k.to_string(), v.clone());
                            }
                        }

                        x => {
                            return Err(ShellError::CantConvert(
                                "string list or single row".into(),
                                x.get_type().to_string(),
                                headers.span().unwrap_or_else(|_| Span::new(0, 0)),
                                None,
                            ));
                        }
                    }
                } else {
                    // primitive values ([key1 val1 key2 val2])
                    for row in table.chunks(2) {
                        if row.len() == 2 {
                            custom_headers.insert(row[0].as_string()?, row[1].clone());
                        }
                    }
                }
            }

            x => {
                return Err(ShellError::CantConvert(
                    "string list or single row".into(),
                    x.get_type().to_string(),
                    headers.span().unwrap_or_else(|_| Span::new(0, 0)),
                    None,
                ));
            }
        };

        for (k, v) in &custom_headers {
            if let Ok(s) = v.as_string() {
                request = request.set(k, &s);
            }
        }
    }

    let response = match body {
        Value::Binary { val, .. } => request.send_bytes(&val),
        Value::String { val, .. } => request.send_string(&val),
        Value::Record { .. } if body_type == BodyType::Json => {
            request.send_json(value_to_json_value(&body)?)
        }
        Value::Record { cols, vals, .. } if body_type == BodyType::Form => {
            let mut data: Vec<(String, String)> = vec![];
            for (col, val) in cols.iter().zip(vals.iter()) {
                data.push((col.clone(), val.as_string()?))
            }
            let data = data
                .iter()
                .map(|(a, b)| (&**a, &**b))
                .collect::<Vec<(&str, &str)>>();
            request.send_form(&data[..])
        }
        Value::List { vals, .. } if body_type == BodyType::Form => {
            if vals.len() % 2 != 0 {
                return Err(ShellError::IOError("unsupported body input".into()));
            }

            let mut data = vec![];
            for chunk in vals.chunks(2) {
                let key = chunk[0].as_string()?;
                let val = chunk[1].as_string()?;
                data.push((key, val));
            }

            let data = data
                .iter()
                .map(|(a, b)| (&**a, &**b))
                .collect::<Vec<(&str, &str)>>();
            request.send_form(&data[..])
        }
        _ => {
            return Err(ShellError::IOError("unsupported body input".into()));
        }
    };

    match response {
        Ok(resp) => {
            let content_type = mime::Mime::from_str(resp.content_type()).map_err(|_| {
                ShellError::GenericError(
                    format!("MIME type unknown: {}", resp.content_type()),
                    "".to_string(),
                    None,
                    Some("given unknown MIME type".to_string()),
                    Vec::new(),
                )
            })?;
            let ext = match (content_type.type_(), content_type.subtype()) {
                (mime::TEXT, mime::PLAIN) => {
                    let path_extension = url::Url::parse(&requested_url)
                        .map_err(|_| {
                            ShellError::GenericError(
                                format!("Cannot parse URL: {}", requested_url),
                                "".to_string(),
                                None,
                                Some("cannot parse".to_string()),
                                Vec::new(),
                            )
                        })?
                        .path_segments()
                        .and_then(|segments| segments.last())
                        .and_then(|name| if name.is_empty() { None } else { Some(name) })
                        .and_then(|name| {
                            PathBuf::from(name)
                                .extension()
                                .map(|name| name.to_string_lossy().to_string())
                        });
                    path_extension
                }
                _ => Some(content_type.subtype().to_string()),
            };
            let output = response_to_buffer(resp, engine_state, span);

            if raw {
                return Ok(output);
            }
            if let Some(ext) = ext {
                match engine_state.find_decl(format!("from {}", ext).as_bytes(), &[]) {
                    Some(converter_id) => engine_state.get_decl(converter_id).run(
                        engine_state,
                        stack,
                        &Call::new(span),
                        output,
                    ),
                    None => Ok(output),
                }
            } else {
                Ok(output)
            }
        }
        Err(Error::Status(err_code, _)) => match err_code {
            404 => Err(ShellError::NetworkFailure(
                format!("Requested file not found (404): {:?}", requested_url),
                span,
            )),
            301 => Err(ShellError::NetworkFailure(
                format!("Resource moved permanently (301): {:?}", requested_url),
                span,
            )),
            400 => Err(ShellError::NetworkFailure(
                format!("Bad request (400) to {:?}", requested_url),
                span,
            )),
            403 => Err(ShellError::NetworkFailure(
                format!("Access forbidden (403) to {:?}", requested_url),
                span,
            )),
            _ => Err(ShellError::NetworkFailure(
                format!(
                    "Cannot make request to {:?}. Error is {:?}",
                    requested_url, err_code
                ),
                span,
            )),
        },
        Err(Error::Transport(transport)) => Err(ShellError::NetworkFailure(
            format!(
                "Cannot make request to {:?}. Error is {:?}",
                requested_url,
                transport.message()
            ),
            span,
        )),
    }
}

fn response_to_buffer(
    response: Response,
    engine_state: &EngineState,
    span: Span,
) -> nu_protocol::PipelineData {
    let reader = response.into_reader();
    let buffered_input = BufReader::new(reader);

    PipelineData::ExternalStream {
        stdout: Some(RawStream::new(
            Box::new(BufferedReader {
                input: buffered_input,
            }),
            engine_state.ctrlc.clone(),
            span,
        )),
        stderr: None,
        exit_code: None,
        span,
        metadata: None,
    }
}
