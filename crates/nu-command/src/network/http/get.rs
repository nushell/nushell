use crate::network::http::client::{
    http_client, request_add_authorization_header, request_add_custom_headers, request_set_timeout,
    response_to_buffer,
};
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, SyntaxShape, Type, Value,
};
use reqwest::StatusCode;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "http get"
    }

    fn signature(&self) -> Signature {
        Signature::build("http get")
            .input_output_types(vec![(Type::Nothing, Type::Any)])
            .required(
                "URL",
                SyntaxShape::String,
                "the URL to fetch the contents from",
            )
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
                "timeout",
                SyntaxShape::Int,
                "timeout period in seconds",
                None,
            )
            .named(
                "headers",
                SyntaxShape::Any,
                "custom headers you want to add ",
                Some('H'),
            )
            .switch(
                "raw",
                "fetch contents as text rather than a table",
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
        "Fetch the contents from a URL."
    }

    fn extra_usage(&self) -> &str {
        "Performs HTTP GET operation."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec![
            "network", "fetch", "pull", "request", "download", "curl", "wget",
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        run_fetch(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "http get content from example.com",
                example: "http get https://www.example.com",
                result: None,
            },
            Example {
                description: "http get content from example.com, with username and password",
                example: "http get -u myuser -p mypass https://www.example.com",
                result: None,
            },
            Example {
                description: "http get content from example.com, with custom header",
                example: "http get -H [my-header-key my-header-value] https://www.example.com",
                result: None,
            },
        ]
    }
}

struct Arguments {
    url: Value,
    raw: bool,
    insecure: Option<bool>,
    user: Option<String>,
    password: Option<String>,
    timeout: Option<Value>,
    headers: Option<Value>,
}

fn run_fetch(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let args = Arguments {
        url: call.req(engine_state, stack, 0)?,
        raw: call.has_flag("raw"),
        insecure: call.get_flag(engine_state, stack, "insecure")?,
        user: call.get_flag(engine_state, stack, "user")?,
        password: call.get_flag(engine_state, stack, "password")?,
        timeout: call.get_flag(engine_state, stack, "timeout")?,
        headers: call.get_flag(engine_state, stack, "headers")?,
    };
    helper(engine_state, stack, args)
}

// Helper function that actually goes to retrieve the resource from the url given
// The Option<String> return a possible file extension which can be used in AutoConvert commands
fn helper(
    engine_state: &EngineState,
    stack: &mut Stack,
    args: Arguments,
) -> std::result::Result<PipelineData, ShellError> {
    // There is no need to error-check this, as the URL is already guaranteed by basic nu command argument type checks.
    let url_value = args.url;

    let span = url_value.span()?;
    let requested_url = url_value.as_string()?;
    let url = match url::Url::parse(&requested_url) {
        Ok(u) => u,
        Err(_e) => {
            return Err(ShellError::TypeMismatch(
                "Incomplete or incorrect URL. Expected a full URL, e.g., https://www.example.com"
                    .to_string(),
                span,
            ));
        }
    };
    let user = args.user.clone();
    let password = args.password;
    let timeout = args.timeout;
    let headers = args.headers;
    let raw = args.raw;

    let client = http_client(args.insecure.is_some());
    let mut request = client.get(url);

    request = request_set_timeout(timeout, request)?;
    request = request_add_authorization_header(user, password, request);
    request = request_add_custom_headers(headers, request)?;

    // Explicitly turn 4xx and 5xx statuses into errors.
    match request.send().and_then(|r| r.error_for_status()) {
        Ok(resp) => match resp.headers().get("content-type") {
            Some(content_type) => {
                let content_type = content_type.to_str().map_err(|e| {
                    ShellError::GenericError(
                        e.to_string(),
                        "".to_string(),
                        None,
                        Some("MIME type were invalid".to_string()),
                        Vec::new(),
                    )
                })?;
                let content_type = mime::Mime::from_str(content_type).map_err(|_| {
                    ShellError::GenericError(
                        format!("MIME type unknown: {content_type}"),
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
                                    format!("Cannot parse URL: {requested_url}"),
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
                    match engine_state.find_decl(format!("from {ext}").as_bytes(), &[]) {
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
        Err(e) if e.is_timeout() => Err(ShellError::NetworkFailure(
            format!("Request to {requested_url} has timed out"),
            span,
        )),
        Err(e) if e.is_status() => match e.status() {
            Some(err_code) if err_code == StatusCode::NOT_FOUND => Err(ShellError::NetworkFailure(
                format!("Requested file not found (404): {requested_url:?}"),
                span,
            )),
            Some(err_code) if err_code == StatusCode::MOVED_PERMANENTLY => {
                Err(ShellError::NetworkFailure(
                    format!("Resource moved permanently (301): {requested_url:?}"),
                    span,
                ))
            }
            Some(err_code) if err_code == StatusCode::BAD_REQUEST => Err(
                ShellError::NetworkFailure(format!("Bad request (400) to {requested_url:?}"), span),
            ),
            Some(err_code) if err_code == StatusCode::FORBIDDEN => Err(ShellError::NetworkFailure(
                format!("Access forbidden (403) to {requested_url:?}"),
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
