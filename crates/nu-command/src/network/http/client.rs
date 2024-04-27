use crate::formats::value_to_json_value;
use base64::{
    alphabet,
    engine::{general_purpose::PAD, GeneralPurpose},
    Engine,
};
use nu_engine::command_prelude::*;
use nu_protocol::{BufferedReader, RawStream};
use std::{
    collections::HashMap,
    io::BufReader,
    path::PathBuf,
    str::FromStr,
    sync::{
        atomic::AtomicBool,
        mpsc::{self, RecvTimeoutError},
        Arc,
    },
    time::Duration,
};
use ureq::{Error, ErrorKind, Request, Response};
use url::Url;

#[derive(PartialEq, Eq)]
pub enum BodyType {
    Json,
    Form,
    Unknown,
}

#[derive(Clone, Copy, PartialEq)]
pub enum RedirectMode {
    Follow,
    Error,
    Manual,
}

pub fn http_client(
    allow_insecure: bool,
    redirect_mode: RedirectMode,
    engine_state: &EngineState,
    stack: &mut Stack,
) -> Result<ureq::Agent, ShellError> {
    let tls = native_tls::TlsConnector::builder()
        .danger_accept_invalid_certs(allow_insecure)
        .build()
        .map_err(|e| ShellError::GenericError {
            error: format!("Failed to build network tls: {}", e),
            msg: String::new(),
            span: None,
            help: None,
            inner: vec![],
        })?;

    let mut agent_builder = ureq::builder()
        .user_agent("nushell")
        .tls_connector(std::sync::Arc::new(tls));

    if let RedirectMode::Manual | RedirectMode::Error = redirect_mode {
        agent_builder = agent_builder.redirects(0);
    }

    if let Some(http_proxy) = retrieve_http_proxy_from_env(engine_state, stack) {
        if let Ok(proxy) = ureq::Proxy::new(http_proxy) {
            agent_builder = agent_builder.proxy(proxy);
        }
    };

    Ok(agent_builder.build())
}

pub fn http_parse_url(
    call: &Call,
    span: Span,
    raw_url: Value,
) -> Result<(String, Url), ShellError> {
    let requested_url = raw_url.coerce_into_string()?;
    let url = match url::Url::parse(&requested_url) {
        Ok(u) => u,
        Err(_e) => {
            return Err(ShellError::UnsupportedInput { msg: "Incomplete or incorrect URL. Expected a full URL, e.g., https://www.example.com"
                    .to_string(), input: format!("value: '{requested_url:?}'"), msg_span: call.head, input_span: span });
        }
    };

    Ok((requested_url, url))
}

pub fn http_parse_redirect_mode(mode: Option<Spanned<String>>) -> Result<RedirectMode, ShellError> {
    mode.map_or(Ok(RedirectMode::Follow), |v| match &v.item[..] {
        "follow" | "f" => Ok(RedirectMode::Follow),
        "error" | "e" => Ok(RedirectMode::Error),
        "manual" | "m" => Ok(RedirectMode::Manual),
        _ => Err(ShellError::TypeMismatch {
            err_message: "Invalid redirect handling mode".to_string(),
            span: v.span,
        }),
    })
}

pub fn response_to_buffer(
    response: Response,
    engine_state: &EngineState,
    span: Span,
) -> PipelineData {
    // Try to get the size of the file to be downloaded.
    // This is helpful to show the progress of the stream.
    let buffer_size = match response.header("content-length") {
        Some(content_length) => {
            let content_length = content_length.parse::<u64>().unwrap_or_default();

            if content_length == 0 {
                None
            } else {
                Some(content_length)
            }
        }
        _ => None,
    };

    let reader = response.into_reader();
    let buffered_input = BufReader::new(reader);

    PipelineData::ExternalStream {
        stdout: Some(RawStream::new(
            Box::new(BufferedReader::new(buffered_input)),
            engine_state.ctrlc.clone(),
            span,
            buffer_size,
        )),
        stderr: None,
        exit_code: None,
        span,
        metadata: None,
        trim_end_newline: false,
    }
}

pub fn request_add_authorization_header(
    user: Option<String>,
    password: Option<String>,
    mut request: Request,
) -> Request {
    let base64_engine = GeneralPurpose::new(&alphabet::STANDARD, PAD);

    let login = match (user, password) {
        (Some(user), Some(password)) => {
            let mut enc_str = String::new();
            base64_engine.encode_string(&format!("{user}:{password}"), &mut enc_str);
            Some(enc_str)
        }
        (Some(user), _) => {
            let mut enc_str = String::new();
            base64_engine.encode_string(&format!("{user}:"), &mut enc_str);
            Some(enc_str)
        }
        (_, Some(password)) => {
            let mut enc_str = String::new();
            base64_engine.encode_string(&format!(":{password}"), &mut enc_str);
            Some(enc_str)
        }
        _ => None,
    };

    if let Some(login) = login {
        request = request.set("Authorization", &format!("Basic {login}"));
    }

    request
}

#[allow(clippy::large_enum_variant)]
pub enum ShellErrorOrRequestError {
    ShellError(ShellError),
    RequestError(String, Box<Error>),
}

impl From<ShellError> for ShellErrorOrRequestError {
    fn from(error: ShellError) -> Self {
        ShellErrorOrRequestError::ShellError(error)
    }
}

pub fn send_request(
    request: Request,
    body: Option<Value>,
    content_type: Option<String>,
    ctrl_c: Option<Arc<AtomicBool>>,
) -> Result<Response, ShellErrorOrRequestError> {
    let request_url = request.url().to_string();
    if body.is_none() {
        return send_cancellable_request(&request_url, Box::new(|| request.call()), ctrl_c);
    }
    let body = body.expect("Should never be none.");

    let body_type = match content_type {
        Some(it) if it == "application/json" => BodyType::Json,
        Some(it) if it == "application/x-www-form-urlencoded" => BodyType::Form,
        _ => BodyType::Unknown,
    };
    match body {
        Value::Binary { val, .. } => send_cancellable_request(
            &request_url,
            Box::new(move || request.send_bytes(&val)),
            ctrl_c,
        ),
        Value::String { .. } if body_type == BodyType::Json => {
            let data = value_to_json_value(&body)?;
            send_cancellable_request(&request_url, Box::new(|| request.send_json(data)), ctrl_c)
        }
        Value::String { val, .. } => send_cancellable_request(
            &request_url,
            Box::new(move || request.send_string(&val)),
            ctrl_c,
        ),
        Value::Record { .. } if body_type == BodyType::Json => {
            let data = value_to_json_value(&body)?;
            send_cancellable_request(&request_url, Box::new(|| request.send_json(data)), ctrl_c)
        }
        Value::Record { val, .. } if body_type == BodyType::Form => {
            let mut data: Vec<(String, String)> = Vec::with_capacity(val.len());

            for (col, val) in val.into_owned() {
                data.push((col, val.coerce_into_string()?))
            }

            let request_fn = move || {
                // coerce `data` into a shape that send_form() is happy with
                let data = data
                    .iter()
                    .map(|(a, b)| (a.as_str(), b.as_str()))
                    .collect::<Vec<(&str, &str)>>();
                request.send_form(&data)
            };
            send_cancellable_request(&request_url, Box::new(request_fn), ctrl_c)
        }
        Value::List { vals, .. } if body_type == BodyType::Form => {
            if vals.len() % 2 != 0 {
                return Err(ShellErrorOrRequestError::ShellError(ShellError::IOError {
                    msg: "unsupported body input".into(),
                }));
            }

            let data = vals
                .chunks(2)
                .map(|it| Ok((it[0].coerce_string()?, it[1].coerce_string()?)))
                .collect::<Result<Vec<(String, String)>, ShellErrorOrRequestError>>()?;

            let request_fn = move || {
                // coerce `data` into a shape that send_form() is happy with
                let data = data
                    .iter()
                    .map(|(a, b)| (a.as_str(), b.as_str()))
                    .collect::<Vec<(&str, &str)>>();
                request.send_form(&data)
            };
            send_cancellable_request(&request_url, Box::new(request_fn), ctrl_c)
        }
        Value::List { .. } if body_type == BodyType::Json => {
            let data = value_to_json_value(&body)?;
            send_cancellable_request(&request_url, Box::new(|| request.send_json(data)), ctrl_c)
        }
        _ => Err(ShellErrorOrRequestError::ShellError(ShellError::IOError {
            msg: "unsupported body input".into(),
        })),
    }
}

// Helper method used to make blocking HTTP request calls cancellable with ctrl+c
// ureq functions can block for a long time (default 30s?) while attempting to make an HTTP connection
fn send_cancellable_request(
    request_url: &str,
    request_fn: Box<dyn FnOnce() -> Result<Response, Error> + Sync + Send>,
    ctrl_c: Option<Arc<AtomicBool>>,
) -> Result<Response, ShellErrorOrRequestError> {
    let (tx, rx) = mpsc::channel::<Result<Response, Error>>();

    // Make the blocking request on a background thread...
    std::thread::Builder::new()
        .name("HTTP requester".to_string())
        .spawn(move || {
            let ret = request_fn();
            let _ = tx.send(ret); // may fail if the user has cancelled the operation
        })
        .map_err(ShellError::from)?;

    // ...and poll the channel for responses
    loop {
        if nu_utils::ctrl_c::was_pressed(&ctrl_c) {
            // Return early and give up on the background thread. The connection will either time out or be disconnected
            return Err(ShellErrorOrRequestError::ShellError(
                ShellError::InterruptedByUser { span: None },
            ));
        }

        // 100ms wait time chosen arbitrarily
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(result) => {
                return result.map_err(|e| {
                    ShellErrorOrRequestError::RequestError(request_url.to_string(), Box::new(e))
                });
            }
            Err(RecvTimeoutError::Timeout) => continue,
            Err(RecvTimeoutError::Disconnected) => panic!("http response channel disconnected"),
        }
    }
}

pub fn request_set_timeout(
    timeout: Option<Value>,
    mut request: Request,
) -> Result<Request, ShellError> {
    if let Some(timeout) = timeout {
        let val = timeout.as_i64()?;
        if val.is_negative() || val < 1 {
            return Err(ShellError::TypeMismatch {
                err_message: "Timeout value must be an int and larger than 0".to_string(),
                span: timeout.span(),
            });
        }

        request = request.timeout(Duration::from_secs(val as u64));
    }

    Ok(request)
}

pub fn request_add_custom_headers(
    headers: Option<Value>,
    mut request: Request,
) -> Result<Request, ShellError> {
    if let Some(headers) = headers {
        let mut custom_headers: HashMap<String, Value> = HashMap::new();

        match &headers {
            Value::Record { val, .. } => {
                for (k, v) in &**val {
                    custom_headers.insert(k.to_string(), v.clone());
                }
            }

            Value::List { vals: table, .. } => {
                if table.len() == 1 {
                    // single row([key1 key2]; [val1 val2])
                    match &table[0] {
                        Value::Record { val, .. } => {
                            for (k, v) in &**val {
                                custom_headers.insert(k.to_string(), v.clone());
                            }
                        }

                        x => {
                            return Err(ShellError::CantConvert {
                                to_type: "string list or single row".into(),
                                from_type: x.get_type().to_string(),
                                span: headers.span(),
                                help: None,
                            });
                        }
                    }
                } else {
                    // primitive values ([key1 val1 key2 val2])
                    for row in table.chunks(2) {
                        if row.len() == 2 {
                            custom_headers.insert(row[0].coerce_string()?, row[1].clone());
                        }
                    }
                }
            }

            x => {
                return Err(ShellError::CantConvert {
                    to_type: "string list or single row".into(),
                    from_type: x.get_type().to_string(),
                    span: headers.span(),
                    help: None,
                });
            }
        };

        for (k, v) in custom_headers {
            if let Ok(s) = v.coerce_into_string() {
                request = request.set(&k, &s);
            }
        }
    }

    Ok(request)
}

fn handle_response_error(span: Span, requested_url: &str, response_err: Error) -> ShellError {
    match response_err {
        Error::Status(301, _) => ShellError::NetworkFailure { msg: format!("Resource moved permanently (301): {requested_url:?}"), span },
        Error::Status(400, _) => {
            ShellError::NetworkFailure { msg: format!("Bad request (400) to {requested_url:?}"), span }
        }
        Error::Status(403, _) => {
            ShellError::NetworkFailure { msg: format!("Access forbidden (403) to {requested_url:?}"), span }
        }
        Error::Status(404, _) => ShellError::NetworkFailure { msg: format!("Requested file not found (404): {requested_url:?}"), span },
        Error::Status(408, _) => {
            ShellError::NetworkFailure { msg: format!("Request timeout (408): {requested_url:?}"), span }
        }
        Error::Status(_, _) => ShellError::NetworkFailure { msg: format!(
                "Cannot make request to {:?}. Error is {:?}",
                requested_url,
                response_err.to_string()
            ), span },

        Error::Transport(t) => match t {
            t if t.kind() == ErrorKind::ConnectionFailed => ShellError::NetworkFailure { msg: format!("Cannot make request to {requested_url}, there was an error establishing a connection.",), span },
            t => ShellError::NetworkFailure { msg: t.to_string(), span },
        },
    }
}

pub struct RequestFlags {
    pub allow_errors: bool,
    pub raw: bool,
    pub full: bool,
}

fn transform_response_using_content_type(
    engine_state: &EngineState,
    stack: &mut Stack,
    span: Span,
    requested_url: &str,
    flags: &RequestFlags,
    resp: Response,
    content_type: &str,
) -> Result<PipelineData, ShellError> {
    let content_type =
        mime::Mime::from_str(content_type).map_err(|_| ShellError::GenericError {
            error: format!("MIME type unknown: {content_type}"),
            msg: "".into(),
            span: None,
            help: Some("given unknown MIME type".into()),
            inner: vec![],
        })?;
    let ext = match (content_type.type_(), content_type.subtype()) {
        (mime::TEXT, mime::PLAIN) => {
            let path_extension = url::Url::parse(requested_url)
                .map_err(|_| ShellError::GenericError {
                    error: format!("Cannot parse URL: {requested_url}"),
                    msg: "".into(),
                    span: None,
                    help: Some("cannot parse".into()),
                    inner: vec![],
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
    if flags.raw {
        Ok(output)
    } else if let Some(ext) = ext {
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

pub fn check_response_redirection(
    redirect_mode: RedirectMode,
    span: Span,
    response: &Result<Response, ShellErrorOrRequestError>,
) -> Result<(), ShellError> {
    if let Ok(resp) = response {
        if RedirectMode::Error == redirect_mode && (300..400).contains(&resp.status()) {
            return Err(ShellError::NetworkFailure {
                msg: format!(
                    "Redirect encountered when redirect handling mode was 'error' ({} {})",
                    resp.status(),
                    resp.status_text()
                ),
                span,
            });
        }
    }
    Ok(())
}

fn request_handle_response_content(
    engine_state: &EngineState,
    stack: &mut Stack,
    span: Span,
    requested_url: &str,
    flags: RequestFlags,
    resp: Response,
    request: Request,
) -> Result<PipelineData, ShellError> {
    // #response_to_buffer moves "resp" making it impossible to read headers later.
    // Wrapping it into a closure to call when needed
    let mut consume_response_body = |response: Response| {
        let content_type = response.header("content-type").map(|s| s.to_owned());

        match content_type {
            Some(content_type) => transform_response_using_content_type(
                engine_state,
                stack,
                span,
                requested_url,
                &flags,
                response,
                &content_type,
            ),
            None => Ok(response_to_buffer(response, engine_state, span)),
        }
    };

    if flags.full {
        let response_status = resp.status();

        let request_headers_value = match headers_to_nu(&extract_request_headers(&request), span) {
            Ok(headers) => headers.into_value(span),
            Err(_) => Value::nothing(span),
        };

        let response_headers_value = match headers_to_nu(&extract_response_headers(&resp), span) {
            Ok(headers) => headers.into_value(span),
            Err(_) => Value::nothing(span),
        };

        let headers = record! {
            "request" => request_headers_value,
            "response" => response_headers_value,
        };

        let full_response = Value::record(
            record! {
                "headers" => Value::record(headers, span),
                "body" => consume_response_body(resp)?.into_value(span),
                "status" => Value::int(response_status as i64, span),
            },
            span,
        );

        Ok(full_response.into_pipeline_data())
    } else {
        Ok(consume_response_body(resp)?)
    }
}

pub fn request_handle_response(
    engine_state: &EngineState,
    stack: &mut Stack,
    span: Span,
    requested_url: &str,
    flags: RequestFlags,
    response: Result<Response, ShellErrorOrRequestError>,
    request: Request,
) -> Result<PipelineData, ShellError> {
    match response {
        Ok(resp) => request_handle_response_content(
            engine_state,
            stack,
            span,
            requested_url,
            flags,
            resp,
            request,
        ),
        Err(e) => match e {
            ShellErrorOrRequestError::ShellError(e) => Err(e),
            ShellErrorOrRequestError::RequestError(_, e) => {
                if flags.allow_errors {
                    if let Error::Status(_, resp) = *e {
                        Ok(request_handle_response_content(
                            engine_state,
                            stack,
                            span,
                            requested_url,
                            flags,
                            resp,
                            request,
                        )?)
                    } else {
                        Err(handle_response_error(span, requested_url, *e))
                    }
                } else {
                    Err(handle_response_error(span, requested_url, *e))
                }
            }
        },
    }
}

type Headers = HashMap<String, Vec<String>>;

fn extract_request_headers(request: &Request) -> Headers {
    request
        .header_names()
        .iter()
        .map(|name| {
            (
                name.clone(),
                request.all(name).iter().map(|e| e.to_string()).collect(),
            )
        })
        .collect()
}

fn extract_response_headers(response: &Response) -> Headers {
    response
        .headers_names()
        .iter()
        .map(|name| {
            (
                name.clone(),
                response.all(name).iter().map(|e| e.to_string()).collect(),
            )
        })
        .collect()
}

fn headers_to_nu(headers: &Headers, span: Span) -> Result<PipelineData, ShellError> {
    let mut vals = Vec::with_capacity(headers.len());

    for (name, values) in headers {
        let is_duplicate = vals.iter().any(|val| {
            if let Value::Record { val, .. } = val {
                if let Some((
                    _col,
                    Value::String {
                        val: header_name, ..
                    },
                )) = val.get_index(0)
                {
                    return name == header_name;
                }
            }
            false
        });
        if !is_duplicate {
            // A single header can hold multiple values
            // This interface is why we needed to check if we've already parsed this header name.
            for str_value in values {
                let record = record! {
                    "name" => Value::string(name, span),
                    "value" => Value::string(str_value, span),
                };
                vals.push(Value::record(record, span));
            }
        }
    }

    Ok(Value::list(vals, span).into_pipeline_data())
}

pub fn request_handle_response_headers(
    span: Span,
    response: Result<Response, ShellErrorOrRequestError>,
) -> Result<PipelineData, ShellError> {
    match response {
        Ok(resp) => headers_to_nu(&extract_response_headers(&resp), span),
        Err(e) => match e {
            ShellErrorOrRequestError::ShellError(e) => Err(e),
            ShellErrorOrRequestError::RequestError(requested_url, e) => {
                Err(handle_response_error(span, &requested_url, *e))
            }
        },
    }
}

fn retrieve_http_proxy_from_env(engine_state: &EngineState, stack: &mut Stack) -> Option<String> {
    stack
        .get_env_var(engine_state, "http_proxy")
        .or(stack.get_env_var(engine_state, "HTTP_PROXY"))
        .or(stack.get_env_var(engine_state, "https_proxy"))
        .or(stack.get_env_var(engine_state, "HTTPS_PROXY"))
        .or(stack.get_env_var(engine_state, "ALL_PROXY"))
        .and_then(|proxy| proxy.coerce_into_string().ok())
}
