use base64::encode;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::RawStream;
use reqwest::{blocking::Response, StatusCode};
use std::path::PathBuf;
use std::str::FromStr;

use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Value,
};
use std::io::{BufRead, BufReader, Read};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "post"
    }

    fn signature(&self) -> Signature {
        Signature::build("post")
            .desc("Post content to a URL and retrieve data as a table if possible.")
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
            .switch(
                "raw",
                "return values as a string instead of a table",
                Some('r'),
            )
            .switch(
                "insecure",
                "allow insecure server connections when using SSL",
                Some('k'),
            )
            .filter()
            .category(Category::Network)
    }
    fn usage(&self) -> &str {
        "Post a body to a URL (HTTP POST operation)."
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
        ]
    }
}

struct Arguments {
    path: Option<Value>,
    body: Option<Value>,
    raw: bool,
    insecure: Option<bool>,
    user: Option<String>,
    password: Option<String>,
    content_type: Option<String>,
    content_length: Option<String>,
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
        raw: call.has_flag("raw"),
        user: call.get_flag(engine_state, stack, "user")?,
        password: call.get_flag(engine_state, stack, "password")?,
        insecure: call.get_flag(engine_state, stack, "insecure")?,
        content_type: call.get_flag(engine_state, stack, "content_type")?,
        content_length: call.get_flag(engine_state, stack, "content_length")?,
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
    let location = url;
    let raw = args.raw;
    let login = match (user, password) {
        (Some(user), Some(password)) => Some(encode(&format!("{}:{}", user, password))),
        (Some(user), _) => Some(encode(&format!("{}:", user))),
        _ => None,
    };
    let mut request = http_client(args.insecure.is_some()).post(location);
    match body {
        Value::Binary { val, .. } => {
            request = request.body(val);
        }
        Value::String { val, .. } => {
            request = request.body(val);
        }
        _ => {
            return Err(ShellError::IOError("unsupported body input".into()));
        }
    };

    if let Some(val) = args.content_type {
        request = request.header("Content-Type", val);
    }
    if let Some(val) = args.content_length {
        request = request.header("Content-Length", val);
    }
    if let Some(login) = login {
        request = request.header("Authorization", format!("Basic {}", login));
    }
    match request.send() {
        Ok(resp) => match resp.headers().get("content-type") {
            Some(content_type) => {
                let content_type = content_type.to_str().map_err(|e| {
                    ShellError::LabeledError(e.to_string(), "MIME type were invalid".to_string())
                })?;
                let content_type = mime::Mime::from_str(content_type).map_err(|_| {
                    ShellError::LabeledError(
                        format!("MIME type unknown: {}", content_type),
                        "given unknown MIME type".to_string(),
                    )
                })?;
                let ext = match (content_type.type_(), content_type.subtype()) {
                    (mime::TEXT, mime::PLAIN) => {
                        let path_extension = url::Url::parse(&requested_url)
                            .map_err(|_| {
                                ShellError::LabeledError(
                                    format!("Cannot parse URL: {}", requested_url),
                                    "cannot parse".to_string(),
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
                    match engine_state.find_decl(format!("from {}", ext).as_bytes()) {
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
            None => Ok(response_to_buffer(resp, engine_state, span)),
        },
        Err(e) if e.is_status() => match e.status() {
            Some(err_code) if err_code == StatusCode::NOT_FOUND => Err(ShellError::NetworkFailure(
                format!("Requested file not found (404): {:?}", requested_url),
                span,
            )),
            Some(err_code) if err_code == StatusCode::MOVED_PERMANENTLY => {
                Err(ShellError::NetworkFailure(
                    format!("Resource moved permanently (301): {:?}", requested_url),
                    span,
                ))
            }
            Some(err_code) if err_code == StatusCode::BAD_REQUEST => {
                Err(ShellError::NetworkFailure(
                    format!("Bad request (400) to {:?}", requested_url),
                    span,
                ))
            }
            Some(err_code) if err_code == StatusCode::FORBIDDEN => Err(ShellError::NetworkFailure(
                format!("Access forbidden (403) to {:?}", requested_url),
                span,
            )),
            _ => Err(ShellError::NetworkFailure(
                format!(
                    "Cannot make request to {:?}. Error is {:?}",
                    requested_url,
                    e.to_string()
                ),
                span,
            )),
        },
        Err(e) => Err(ShellError::NetworkFailure(
            format!(
                "Cannot make request to {:?}. Error is {:?}",
                requested_url,
                e.to_string()
            ),
            span,
        )),
    }
}

pub struct BufferedReader<R: Read> {
    input: BufReader<R>,
}

impl<R: Read> Iterator for BufferedReader<R> {
    type Item = Result<Vec<u8>, ShellError>;

    fn next(&mut self) -> Option<Self::Item> {
        let buffer = self.input.fill_buf();
        match buffer {
            Ok(s) => {
                let result = s.to_vec();

                let buffer_len = s.len();

                if buffer_len == 0 {
                    None
                } else {
                    self.input.consume(buffer_len);

                    Some(Ok(result))
                }
            }
            Err(e) => Some(Err(ShellError::IOError(e.to_string()))),
        }
    }
}
fn response_to_buffer(
    response: Response,
    engine_state: &EngineState,
    span: Span,
) -> nu_protocol::PipelineData {
    let buffered_input = BufReader::new(response);

    PipelineData::RawStream(
        RawStream::new(
            Box::new(BufferedReader {
                input: buffered_input,
            }),
            engine_state.ctrlc.clone(),
            span,
        ),
        span,
        None,
    )
}
// Only panics if the user agent is invalid but we define it statically so either
// it always or never fails
#[allow(clippy::unwrap_used)]
fn http_client(allow_insecure: bool) -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .user_agent("nushell")
        .danger_accept_invalid_certs(allow_insecure)
        .build()
        .expect("Failed to build reqwest client")
}
