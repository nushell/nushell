use crate::{formats::value_to_json_value, network::tls::tls};
use base64::{
    Engine, alphabet,
    engine::{GeneralPurpose, general_purpose::PAD},
};
use multipart_rs::MultipartWriter;
use nu_engine::command_prelude::*;
use nu_protocol::{ByteStream, LabeledError, Signals, shell_error::io::IoError};
use serde_json::Value as JsonValue;
use std::{
    collections::HashMap,
    error::Error as StdError,
    io::Cursor,
    path::PathBuf,
    str::FromStr,
    sync::mpsc::{self, RecvTimeoutError},
    time::Duration,
};
use ureq::{Error, ErrorKind, Request, Response};
use url::Url;

const HTTP_DOCS: &str = "https://www.nushell.sh/cookbook/http.html";

type ContentType = String;

#[derive(Debug, PartialEq, Eq)]
pub enum BodyType {
    Json,
    Form,
    Multipart,
    Unknown(Option<ContentType>),
}

impl From<Option<ContentType>> for BodyType {
    fn from(content_type: Option<ContentType>) -> Self {
        match content_type {
            Some(it) if it.contains("application/json") => BodyType::Json,
            Some(it) if it.contains("application/x-www-form-urlencoded") => BodyType::Form,
            Some(it) if it.contains("multipart/form-data") => BodyType::Multipart,
            Some(it) => BodyType::Unknown(Some(it)),
            None => BodyType::Unknown(None),
        }
    }
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
    let mut agent_builder = ureq::builder()
        .user_agent("nushell")
        .tls_connector(std::sync::Arc::new(tls(allow_insecure)?));

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
    let mut requested_url = raw_url.coerce_into_string()?;
    if requested_url.starts_with(':') {
        requested_url = format!("http://localhost{requested_url}");
    } else if !requested_url.contains("://") {
        requested_url = format!("http://{requested_url}");
    }

    let url = match url::Url::parse(&requested_url) {
        Ok(u) => u,
        Err(_e) => {
            return Err(ShellError::UnsupportedInput {
                msg: "Incomplete or incorrect URL. Expected a full URL, e.g., https://www.example.com".to_string(),
                input: format!("value: '{requested_url:?}'"),
                msg_span: call.head,
                input_span: span
            });
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

    // Try to guess whether the response is definitely intended to binary or definitely intended to
    // be UTF-8 text. Otherwise specify `None` and just guess. This doesn't have to be thorough.
    let content_type_lowercase = response.header("content-type").map(|s| s.to_lowercase());
    let response_type = match content_type_lowercase.as_deref() {
        Some("application/octet-stream") => ByteStreamType::Binary,
        Some(h) if h.contains("charset=utf-8") => ByteStreamType::String,
        _ => ByteStreamType::Unknown,
    };

    let reader = response.into_reader();

    PipelineData::ByteStream(
        ByteStream::read(reader, span, engine_state.signals().clone(), response_type)
            .with_known_size(buffer_size),
        None,
    )
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
            base64_engine.encode_string(format!("{user}:{password}"), &mut enc_str);
            Some(enc_str)
        }
        (Some(user), _) => {
            let mut enc_str = String::new();
            base64_engine.encode_string(format!("{user}:"), &mut enc_str);
            Some(enc_str)
        }
        (_, Some(password)) => {
            let mut enc_str = String::new();
            base64_engine.encode_string(format!(":{password}"), &mut enc_str);
            Some(enc_str)
        }
        _ => None,
    };

    if let Some(login) = login {
        request = request.set("Authorization", &format!("Basic {login}"));
    }

    request
}

#[derive(Debug)]
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

#[derive(Debug)]
pub enum HttpBody {
    Value(Value),
    ByteStream(ByteStream),
    None,
}

// remove once all commands have been migrated
pub fn send_request(
    engine_state: &EngineState,
    request: Request,
    http_body: HttpBody,
    content_type: Option<String>,
    span: Span,
    signals: &Signals,
) -> Result<Response, ShellErrorOrRequestError> {
    let request_url = request.url().to_string();
    // hard code serialze_types to false because closures probably shouldn't be
    // deserialized for send_request but it's required by send_json_request
    let serialze_types = false;

    match http_body {
        HttpBody::None => {
            send_cancellable_request(&request_url, Box::new(|| request.call()), span, signals)
        }
        HttpBody::ByteStream(byte_stream) => {
            let req = if let Some(content_type) = content_type {
                request.set("Content-Type", &content_type)
            } else {
                request
            };

            send_cancellable_request_bytes(&request_url, req, byte_stream, span, signals)
        }
        HttpBody::Value(body) => {
            let body_type = BodyType::from(content_type);

            // We should set the content_type if there is one available
            // when the content type is unknown
            let req = if let BodyType::Unknown(Some(content_type)) = &body_type {
                request.clone().set("Content-Type", content_type)
            } else {
                request
            };

            match body_type {
                BodyType::Json => send_json_request(
                    engine_state,
                    &request_url,
                    body,
                    req,
                    span,
                    signals,
                    serialze_types,
                ),
                BodyType::Form => send_form_request(&request_url, body, req, span, signals),
                BodyType::Multipart => {
                    send_multipart_request(&request_url, body, req, span, signals)
                }
                BodyType::Unknown(_) => {
                    send_default_request(&request_url, body, req, span, signals)
                }
            }
        }
    }
}

fn send_json_request(
    engine_state: &EngineState,
    request_url: &str,
    body: Value,
    req: Request,
    span: Span,
    signals: &Signals,
    serialize_types: bool,
) -> Result<Response, ShellErrorOrRequestError> {
    match body {
        Value::Int { .. } | Value::Float { .. } | Value::List { .. } | Value::Record { .. } => {
            let data = value_to_json_value(engine_state, &body, span, serialize_types)?;
            send_cancellable_request(request_url, Box::new(|| req.send_json(data)), span, signals)
        }
        // If the body type is string, assume it is string json content.
        // If parsing fails, just send the raw string
        Value::String { val: s, .. } => {
            if let Ok(jvalue) = serde_json::from_str::<JsonValue>(&s) {
                send_cancellable_request(
                    request_url,
                    Box::new(|| req.send_json(jvalue)),
                    span,
                    signals,
                )
            } else {
                let data = serde_json::from_str(&s).unwrap_or_else(|_| nu_json::Value::String(s));
                send_cancellable_request(
                    request_url,
                    Box::new(|| req.send_json(data)),
                    span,
                    signals,
                )
            }
        }
        _ => Err(ShellErrorOrRequestError::ShellError(
            ShellError::TypeMismatch {
                err_message: format!(
                    "Accepted types: [int, float, list, string, record]. Check: {HTTP_DOCS}"
                ),
                span: body.span(),
            },
        )),
    }
}

fn send_form_request(
    request_url: &str,
    body: Value,
    req: Request,
    span: Span,
    signals: &Signals,
) -> Result<Response, ShellErrorOrRequestError> {
    let build_request_fn = |data: Vec<(String, String)>| {
        // coerce `data` into a shape that send_form() is happy with
        let data = data
            .iter()
            .map(|(a, b)| (a.as_str(), b.as_str()))
            .collect::<Vec<(&str, &str)>>();
        req.send_form(&data)
    };

    match body {
        Value::List { ref vals, .. } => {
            if vals.len() % 2 != 0 {
                return Err(ShellErrorOrRequestError::ShellError(ShellError::IncorrectValue {
                    msg: "Body type 'list' for form requests requires paired values. E.g.: [foo, 10]".into(),
                    val_span: body.span(),
                    call_span: span,
                }));
            }

            let data = vals
                .chunks(2)
                .map(|it| Ok((it[0].coerce_string()?, it[1].coerce_string()?)))
                .collect::<Result<Vec<(String, String)>, ShellErrorOrRequestError>>()?;

            let request_fn = Box::new(|| build_request_fn(data));
            send_cancellable_request(request_url, request_fn, span, signals)
        }
        Value::Record { val, .. } => {
            let mut data: Vec<(String, String)> = Vec::with_capacity(val.len());

            for (col, val) in val.into_owned() {
                data.push((col, val.coerce_into_string()?))
            }

            let request_fn = Box::new(|| build_request_fn(data));
            send_cancellable_request(request_url, request_fn, span, signals)
        }
        _ => Err(ShellErrorOrRequestError::ShellError(
            ShellError::TypeMismatch {
                err_message: format!("Accepted types: [list, record]. Check: {HTTP_DOCS}"),
                span: body.span(),
            },
        )),
    }
}

fn send_multipart_request(
    request_url: &str,
    body: Value,
    req: Request,
    span: Span,
    signals: &Signals,
) -> Result<Response, ShellErrorOrRequestError> {
    let request_fn = match body {
        Value::Record { val, .. } => {
            let mut builder = MultipartWriter::new();

            let err = |e: std::io::Error| {
                ShellErrorOrRequestError::ShellError(IoError::new(e, span, None).into())
            };

            for (col, val) in val.into_owned() {
                if let Value::Binary { val, .. } = val {
                    let headers = [
                        "Content-Type: application/octet-stream".to_string(),
                        "Content-Transfer-Encoding: binary".to_string(),
                        format!(
                            "Content-Disposition: form-data; name=\"{col}\"; filename=\"{col}\""
                        ),
                        format!("Content-Length: {}", val.len()),
                    ];
                    builder
                        .add(&mut Cursor::new(val), &headers.join("\r\n"))
                        .map_err(err)?;
                } else {
                    let headers = format!(r#"Content-Disposition: form-data; name="{col}""#);
                    builder
                        .add(val.coerce_into_string()?.as_bytes(), &headers)
                        .map_err(err)?;
                }
            }
            builder.finish();

            let (boundary, data) = (builder.boundary, builder.data);
            let content_type = format!("multipart/form-data; boundary={boundary}");

            move || req.set("Content-Type", &content_type).send_bytes(&data)
        }
        _ => {
            return Err(ShellErrorOrRequestError::ShellError(
                ShellError::TypeMismatch {
                    err_message: format!("Accepted types: [record]. Check: {HTTP_DOCS}"),
                    span: body.span(),
                },
            ));
        }
    };
    send_cancellable_request(request_url, Box::new(request_fn), span, signals)
}

fn send_default_request(
    request_url: &str,
    body: Value,
    req: Request,
    span: Span,
    signals: &Signals,
) -> Result<Response, ShellErrorOrRequestError> {
    match body {
        Value::Binary { val, .. } => send_cancellable_request(
            request_url,
            Box::new(move || req.send_bytes(&val)),
            span,
            signals,
        ),
        Value::String { val, .. } => send_cancellable_request(
            request_url,
            Box::new(move || req.send_string(&val)),
            span,
            signals,
        ),
        _ => Err(ShellErrorOrRequestError::ShellError(
            ShellError::TypeMismatch {
                err_message: format!("Accepted types: [binary, string]. Check: {HTTP_DOCS}"),
                span: body.span(),
            },
        )),
    }
}

// Helper method used to make blocking HTTP request calls cancellable with ctrl+c
// ureq functions can block for a long time (default 30s?) while attempting to make an HTTP connection
fn send_cancellable_request(
    request_url: &str,
    request_fn: Box<dyn FnOnce() -> Result<Response, Error> + Sync + Send>,
    span: Span,
    signals: &Signals,
) -> Result<Response, ShellErrorOrRequestError> {
    let (tx, rx) = mpsc::channel::<Result<Response, Error>>();

    // Make the blocking request on a background thread...
    std::thread::Builder::new()
        .name("HTTP requester".to_string())
        .spawn(move || {
            let ret = request_fn();
            let _ = tx.send(ret); // may fail if the user has cancelled the operation
        })
        .map_err(|err| {
            IoError::new_with_additional_context(err, span, None, "Could not spawn HTTP requester")
        })
        .map_err(ShellError::from)?;

    // ...and poll the channel for responses
    loop {
        signals.check(&span)?;

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

// Helper method used to make blocking HTTP request calls cancellable with ctrl+c
// ureq functions can block for a long time (default 30s?) while attempting to make an HTTP connection
fn send_cancellable_request_bytes(
    request_url: &str,
    request: Request,
    byte_stream: ByteStream,
    span: Span,
    signals: &Signals,
) -> Result<Response, ShellErrorOrRequestError> {
    let (tx, rx) = mpsc::channel::<Result<Response, ShellErrorOrRequestError>>();
    let request_url_string = request_url.to_string();

    // Make the blocking request on a background thread...
    std::thread::Builder::new()
        .name("HTTP requester".to_string())
        .spawn(move || {
            let ret = byte_stream
                .reader()
                .ok_or_else(|| {
                    ShellErrorOrRequestError::ShellError(ShellError::GenericError {
                        error: "Could not read byte stream".to_string(),
                        msg: "".into(),
                        span: None,
                        help: None,
                        inner: vec![],
                    })
                })
                .and_then(|reader| {
                    request.send(reader).map_err(|e| {
                        ShellErrorOrRequestError::RequestError(request_url_string, Box::new(e))
                    })
                });

            // may fail if the user has cancelled the operation
            let _ = tx.send(ret);
        })
        .map_err(|err| {
            IoError::new_with_additional_context(err, span, None, "Could not spawn HTTP requester")
        })
        .map_err(ShellError::from)?;

    // ...and poll the channel for responses
    loop {
        signals.check(&span)?;

        // 100ms wait time chosen arbitrarily
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(result) => return result,
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
        let val = timeout.as_duration()?;
        if val.is_negative() || val < 1 {
            return Err(ShellError::TypeMismatch {
                err_message: "Timeout value must be an int and larger than 0".to_string(),
                span: timeout.span(),
            });
        }

        request = request.timeout(Duration::from_nanos(val as u64));
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
        Error::Status(301, _) => ShellError::NetworkFailure {
            msg: format!("Resource moved permanently (301): {requested_url:?}"),
            span,
        },
        Error::Status(400, _) => ShellError::NetworkFailure {
            msg: format!("Bad request (400) to {requested_url:?}"),
            span,
        },
        Error::Status(403, _) => ShellError::NetworkFailure {
            msg: format!("Access forbidden (403) to {requested_url:?}"),
            span,
        },
        Error::Status(404, _) => ShellError::NetworkFailure {
            msg: format!("Requested file not found (404): {requested_url:?}"),
            span,
        },
        Error::Status(408, _) => ShellError::NetworkFailure {
            msg: format!("Request timeout (408): {requested_url:?}"),
            span,
        },
        Error::Status(_, _) => ShellError::NetworkFailure {
            msg: format!(
                "Cannot make request to {:?}. Error is {:?}",
                requested_url,
                response_err.to_string()
            ),
            span,
        },

        Error::Transport(t) => {
            let generic_network_failure = || ShellError::NetworkFailure {
                msg: t.to_string(),
                span,
            };
            match t.kind() {
                ErrorKind::ConnectionFailed => ShellError::NetworkFailure {
                    msg: format!(
                        "Cannot make request to {requested_url}, there was an error establishing a connection.",
                    ),
                    span,
                },
                ErrorKind::Io => 'io: {
                    let Some(source) = t.source() else {
                        break 'io generic_network_failure();
                    };

                    let Some(io_error) = source.downcast_ref::<std::io::Error>() else {
                        break 'io generic_network_failure();
                    };

                    ShellError::Io(IoError::new(io_error, span, None))
                }
                _ => generic_network_failure(),
            }
        }
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
    let content_type = mime::Mime::from_str(content_type)
        // there are invalid content types in the wild, so we try to recover
        // Example: `Content-Type: "text/plain"; charset="utf8"` (note the quotes)
        .or_else(|_| mime::Mime::from_str(&content_type.replace('"', "")))
        .or_else(|_| mime::Mime::from_str("text/plain"))
        .expect("Failed to parse content type, and failed to default to text/plain");

    let ext = match (content_type.type_(), content_type.subtype()) {
        (mime::TEXT, mime::PLAIN) => url::Url::parse(requested_url)
            .map_err(|err| {
                LabeledError::new(err.to_string())
                    .with_help("cannot parse")
                    .with_label(
                        format!("Cannot parse URL: {requested_url}"),
                        Span::unknown(),
                    )
            })?
            .path_segments()
            .and_then(|mut segments| segments.next_back())
            .and_then(|name| if name.is_empty() { None } else { Some(name) })
            .and_then(|name| {
                PathBuf::from(name)
                    .extension()
                    .map(|name| name.to_string_lossy().to_string())
            }),
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

        let request_headers_value = headers_to_nu(&extract_request_headers(&request), span)
            .and_then(|data| data.into_value(span))
            .unwrap_or(Value::nothing(span));

        let response_headers_value = headers_to_nu(&extract_response_headers(&resp), span)
            .and_then(|data| data.into_value(span))
            .unwrap_or(Value::nothing(span));

        let headers = record! {
            "request" => request_headers_value,
            "response" => response_headers_value,
        };

        let body = consume_response_body(resp)?.into_value(span)?;

        let full_response = Value::record(
            record! {
                "headers" => Value::record(headers, span),
                "body" => body,
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
        .cloned()
        .and_then(|proxy| proxy.coerce_into_string().ok())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_body_type_from_content_type() {
        let json = Some("application/json".to_string());
        assert_eq!(BodyType::Json, BodyType::from(json));

        // while the charset wont' be passed as we are allowing serde and the library to control
        // this, it still shouldn't be missed as json if passed in.
        let json_with_charset = Some("application/json; charset=utf-8".to_string());
        assert_eq!(BodyType::Json, BodyType::from(json_with_charset));

        let form = Some("application/x-www-form-urlencoded".to_string());
        assert_eq!(BodyType::Form, BodyType::from(form));

        let multipart = Some("multipart/form-data".to_string());
        assert_eq!(BodyType::Multipart, BodyType::from(multipart));

        let unknown = Some("application/octet-stream".to_string());
        assert_eq!(BodyType::Unknown(unknown.clone()), BodyType::from(unknown));

        let none = None;
        assert_eq!(BodyType::Unknown(none.clone()), BodyType::from(none));
    }
}
