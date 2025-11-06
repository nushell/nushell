use crate::{
    formats::value_to_json_value,
    network::{http::timeout_extractor_reader::UreqTimeoutExtractorReader, tls::tls_config},
};
use base64::{
    Engine, alphabet,
    engine::{GeneralPurpose, general_purpose::PAD},
};
use http::StatusCode;
use log::error;
use multipart_rs::MultipartWriter;
use nu_engine::command_prelude::*;
use nu_protocol::{ByteStream, LabeledError, PipelineMetadata, Signals, shell_error::io::IoError};
use serde_json::Value as JsonValue;
use std::{
    collections::HashMap,
    io::Cursor,
    path::PathBuf,
    str::FromStr,
    sync::mpsc::{self, RecvTimeoutError},
    time::Duration,
};
use ureq::{
    Body, Error, RequestBuilder, ResponseExt, SendBody,
    typestate::{WithBody, WithoutBody},
};
use url::Url;

use crate::network::http::unix_socket::UnixSocketConnector;

const HTTP_DOCS: &str = "https://www.nushell.sh/cookbook/http.html";

type Response = http::Response<Body>;

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

trait GetHeader {
    fn header(&self, key: &str) -> Option<&str>;
}

impl GetHeader for Response {
    fn header(&self, key: &str) -> Option<&str> {
        self.headers().get(key).and_then(|v| {
            v.to_str()
                .map_err(|e| log::error!("Invalid header {e:?}"))
                .ok()
        })
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum RedirectMode {
    Follow,
    Error,
    Manual,
}

impl RedirectMode {
    pub(crate) const MODES: &[&str] = &["follow", "error", "manual"];
}

/// Helper function to add the --unix-socket flag to command signatures.
pub fn add_unix_socket_flag(sig: Signature) -> Signature {
    sig.named(
        "unix-socket",
        SyntaxShape::Filepath,
        "Connect to the specified Unix socket instead of using TCP",
        Some('U'),
    )
}

pub fn http_client(
    allow_insecure: bool,
    redirect_mode: RedirectMode,
    unix_socket_path: Option<PathBuf>,
    engine_state: &EngineState,
    stack: &mut Stack,
) -> Result<ureq::Agent, ShellError> {
    let mut config_builder = ureq::config::Config::builder()
        .user_agent("nushell")
        .save_redirect_history(true)
        .http_status_as_error(false)
        .max_redirects_will_error(false);

    if let RedirectMode::Manual | RedirectMode::Error = redirect_mode {
        config_builder = config_builder.max_redirects(0);
    }

    if let Some(http_proxy) = retrieve_http_proxy_from_env(engine_state, stack)
        && let Ok(proxy) = ureq::Proxy::new(&http_proxy)
    {
        config_builder = config_builder.proxy(Some(proxy));
    };

    config_builder = config_builder.tls_config(tls_config(allow_insecure)?);

    if let Some(socket_path) = unix_socket_path {
        // Use custom Unix socket connector
        use ureq::unversioned::resolver::DefaultResolver;

        let connector = UnixSocketConnector::new(socket_path);
        let config = config_builder.build();
        let resolver = DefaultResolver::default();

        return Ok(ureq::Agent::with_parts(config, connector, resolver));
    }

    Ok(ureq::Agent::new_with_config(config_builder.build()))
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
                input_span: span,
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

    // Extract response metadata before consuming the body
    let metadata = extract_response_metadata(&response, span);

    let reader = UreqTimeoutExtractorReader {
        r: response.into_body().into_reader(),
    };

    PipelineData::byte_stream(
        ByteStream::read(reader, span, engine_state.signals().clone(), response_type)
            .with_known_size(buffer_size),
        Some(metadata),
    )
}

fn extract_response_metadata(response: &Response, span: Span) -> PipelineMetadata {
    let status = Value::int(response.status().as_u16().into(), span);

    let headers_value = headers_to_nu(&extract_response_headers(response), span)
        .and_then(|data| data.into_value(span))
        .unwrap_or(Value::nothing(span));

    let urls = Value::list(
        response
            .get_redirect_history()
            .into_iter()
            .flatten()
            .map(|v| Value::string(v.to_string(), span))
            .collect(),
        span,
    );

    let http_response = Value::record(
        record! {
            "status" => status,
            "headers" => headers_value,
            "urls" => urls,
        },
        span,
    );

    let mut metadata = PipelineMetadata::default();
    metadata
        .custom
        .insert("http_response".to_string(), http_response);
    metadata
}

pub fn request_add_authorization_header<B>(
    user: Option<String>,
    password: Option<String>,
    mut request: RequestBuilder<B>,
) -> RequestBuilder<B> {
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
        request = request.header("Authorization", &format!("Basic {login}"));
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
}

pub fn send_request_no_body(
    request: RequestBuilder<WithoutBody>,
    span: Span,
    signals: &Signals,
) -> (Result<Response, ShellError>, Headers) {
    let headers = extract_request_headers(&request);
    let request_url = request.uri_ref().cloned().unwrap_or_default().to_string();
    let result = send_cancellable_request(&request_url, Box::new(|| request.call()), span, signals)
        .map_err(|e| request_error_to_shell_error(span, e));

    (result, headers.unwrap_or_default())
}

// remove once all commands have been migrated
pub fn send_request(
    engine_state: &EngineState,
    request: RequestBuilder<WithBody>,
    body: HttpBody,
    content_type: Option<String>,
    span: Span,
    signals: &Signals,
) -> (Result<Response, ShellError>, Headers) {
    let mut request_headers = Headers::new();
    let request_url = request.uri_ref().cloned().unwrap_or_default().to_string();
    // hard code serialize_types to false because closures probably shouldn't be
    // deserialized for send_request but it's required by send_json_request
    let serialize_types = false;
    let response = match body {
        HttpBody::ByteStream(byte_stream) => {
            let req = if let Some(content_type) = content_type {
                request.header("Content-Type", &content_type)
            } else {
                request
            };
            if let Some(h) = extract_request_headers(&req) {
                request_headers = h;
            }
            send_cancellable_request_bytes(&request_url, req, byte_stream, span, signals)
        }
        HttpBody::Value(body) => {
            let body_type = BodyType::from(content_type);

            // We should set the content_type if there is one available
            // when the content type is unknown
            let req = if let BodyType::Unknown(Some(content_type)) = &body_type {
                request.header("Content-Type", content_type)
            } else {
                request
            };

            if let Some(h) = extract_request_headers(&req) {
                request_headers = h;
            }

            match body_type {
                BodyType::Json => send_json_request(
                    engine_state,
                    &request_url,
                    body,
                    req,
                    span,
                    signals,
                    serialize_types,
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
    };

    let response = response.map_err(|e| request_error_to_shell_error(span, e));

    (response, request_headers)
}

fn send_json_request(
    engine_state: &EngineState,
    request_url: &str,
    body: Value,
    req: RequestBuilder<WithBody>,
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
    req: RequestBuilder<WithBody>,
    span: Span,
    signals: &Signals,
) -> Result<Response, ShellErrorOrRequestError> {
    let build_request_fn = |data: Vec<(String, String)>| {
        // coerce `data` into a shape that send_form() is happy with
        let data = data
            .iter()
            .map(|(a, b)| (a.as_str(), b.as_str()))
            .collect::<Vec<(&str, &str)>>();
        req.send_form(data)
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
    req: RequestBuilder<WithBody>,
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

            move || req.header("Content-Type", &content_type).send(&data)
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
    req: RequestBuilder<WithBody>,
    span: Span,
    signals: &Signals,
) -> Result<Response, ShellErrorOrRequestError> {
    match body {
        Value::Binary { val, .. } => {
            send_cancellable_request(request_url, Box::new(move || req.send(&val)), span, signals)
        }
        Value::String { val, .. } => {
            send_cancellable_request(request_url, Box::new(move || req.send(&val)), span, signals)
        }
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
    request: ureq::RequestBuilder<WithBody>,
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
                    request
                        .send(SendBody::from_owned_reader(reader))
                        .map_err(|e| {
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

pub fn request_set_timeout<B>(
    timeout: Option<Value>,
    mut request: RequestBuilder<B>,
) -> Result<RequestBuilder<B>, ShellError> {
    if let Some(timeout) = timeout {
        let val = timeout.as_duration()?;
        if val.is_negative() || val < 1 {
            return Err(ShellError::TypeMismatch {
                err_message: "Timeout value must be an int and larger than 0".to_string(),
                span: timeout.span(),
            });
        }

        request = request
            .config()
            .timeout_global(Some(Duration::from_nanos(val as u64)))
            .build()
    }

    Ok(request)
}

pub fn request_add_custom_headers<B>(
    headers: Option<Value>,
    mut request: RequestBuilder<B>,
) -> Result<RequestBuilder<B>, ShellError> {
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
                request = request.header(&k, &s);
            }
        }
    }

    Ok(request)
}

fn handle_status_error(span: Span, requested_url: &str, status: StatusCode) -> ShellError {
    match status {
        StatusCode::MOVED_PERMANENTLY => ShellError::NetworkFailure {
            msg: format!("Resource moved permanently (301): {requested_url:?}"),
            span,
        },
        StatusCode::BAD_REQUEST => ShellError::NetworkFailure {
            msg: format!("Bad request (400) to {requested_url:?}"),
            span,
        },
        StatusCode::FORBIDDEN => ShellError::NetworkFailure {
            msg: format!("Access forbidden (403) to {requested_url:?}"),
            span,
        },
        StatusCode::NOT_FOUND => ShellError::NetworkFailure {
            msg: format!("Requested file not found (404): {requested_url:?}"),
            span,
        },
        StatusCode::REQUEST_TIMEOUT => ShellError::NetworkFailure {
            msg: format!("Request timeout (408): {requested_url:?}"),
            span,
        },
        c => ShellError::NetworkFailure {
            msg: format!(
                "Cannot make request to {:?}. Error is {:?}",
                requested_url,
                c.to_string()
            ),
            span,
        },
    }
}

fn handle_response_error(span: Span, requested_url: &str, response_err: Error) -> ShellError {
    match response_err {
        Error::ConnectionFailed => ShellError::NetworkFailure {
            msg: format!(
                "Cannot make request to {requested_url}, there was an error establishing a connection.",
            ),
            span,
        },
        Error::Timeout(..) => ShellError::Io(IoError::new(
            ErrorKind::from_std(std::io::ErrorKind::TimedOut),
            span,
            None,
        )),
        Error::Io(error) => ShellError::Io(IoError::new(error, span, None)),
        e => ShellError::NetworkFailure {
            msg: e.to_string(),
            span,
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
    resp: &Response,
) -> Result<(), ShellError> {
    if RedirectMode::Error == redirect_mode && (300..400).contains(&resp.status().as_u16()) {
        return Err(ShellError::NetworkFailure {
            msg: format!(
                "Redirect encountered when redirect handling mode was 'error' ({})",
                resp.status()
            ),
            span,
        });
    }

    Ok(())
}

pub(crate) fn handle_response_status(
    resp: &Response,
    redirect_mode: RedirectMode,
    requested_url: &str,
    span: Span,
    allow_errors: bool,
) -> Result<(), ShellError> {
    let manual_redirect = redirect_mode == RedirectMode::Manual;

    let is_success = resp.status().is_success()
        || allow_errors
        || (resp.status().is_redirection() && manual_redirect);
    if is_success {
        Ok(())
    } else {
        Err(handle_status_error(span, requested_url, resp.status()))
    }
}

pub(crate) struct RequestMetadata<'a> {
    pub requested_url: &'a str,
    pub span: Span,
    pub headers: Headers,
    pub redirect_mode: RedirectMode,
    pub flags: RequestFlags,
}

pub(crate) fn request_handle_response(
    engine_state: &EngineState,
    stack: &mut Stack,
    RequestMetadata {
        requested_url,
        span,
        headers,
        redirect_mode,
        flags,
    }: RequestMetadata,

    resp: Response,
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
    handle_response_status(
        &resp,
        redirect_mode,
        requested_url,
        span,
        flags.allow_errors,
    )?;

    if flags.full {
        let response_status = resp.status();

        let request_headers_value = headers_to_nu(&headers, span)
            .and_then(|data| data.into_value(span))
            .unwrap_or(Value::nothing(span));

        let response_headers_value = headers_to_nu(&extract_response_headers(&resp), span)
            .and_then(|data| data.into_value(span))
            .unwrap_or(Value::nothing(span));

        let headers = record! {
            "request" => request_headers_value,
            "response" => response_headers_value,
        };
        let urls = Value::list(
            resp.get_redirect_history()
                .into_iter()
                .flatten()
                .map(|v| Value::string(v.to_string(), span))
                .collect(),
            span,
        );
        let body = consume_response_body(resp)?.into_value(span)?;

        let full_response = Value::record(
            record! {
                "urls" => urls,
                "headers" => Value::record(headers, span),
                "body" => body,
                "status" => Value::int(response_status.as_u16().into(), span),

            },
            span,
        );

        Ok(full_response.into_pipeline_data())
    } else {
        Ok(consume_response_body(resp)?)
    }
}

type Headers = HashMap<String, Vec<String>>;

fn extract_request_headers<B>(request: &RequestBuilder<B>) -> Option<Headers> {
    let headers = request.headers_ref()?;
    let headers_str = headers
        .keys()
        .map(|name| {
            (
                name.to_string().clone(),
                headers
                    .get_all(name)
                    .iter()
                    .filter_map(|v| {
                        v.to_str()
                            .map_err(|e| {
                                error!("Invalid header {name:?}: {e:?}");
                            })
                            .ok()
                            .map(|s| s.to_string())
                    })
                    .collect(),
            )
        })
        .collect();
    Some(headers_str)
}

pub(crate) fn extract_response_headers(response: &Response) -> Headers {
    let header_map = response.headers();
    header_map
        .keys()
        .map(|name| {
            (
                name.to_string().clone(),
                header_map
                    .get_all(name)
                    .iter()
                    .filter_map(|v| {
                        v.to_str()
                            .map_err(|e| {
                                error!("Invalid header {name:?}: {e:?}");
                            })
                            .ok()
                            .map(|s| s.to_string())
                    })
                    .collect(),
            )
        })
        .collect()
}

pub(crate) fn headers_to_nu(headers: &Headers, span: Span) -> Result<PipelineData, ShellError> {
    let mut vals = Vec::with_capacity(headers.len());

    for (name, values) in headers {
        let is_duplicate = vals.iter().any(|val| {
            if let Value::Record { val, .. } = val
                && let Some((
                    _col,
                    Value::String {
                        val: header_name, ..
                    },
                )) = val.get_index(0)
            {
                return name == header_name;
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

pub(crate) fn request_error_to_shell_error(span: Span, e: ShellErrorOrRequestError) -> ShellError {
    match e {
        ShellErrorOrRequestError::ShellError(e) => e,
        ShellErrorOrRequestError::RequestError(requested_url, e) => {
            handle_response_error(span, &requested_url, *e)
        }
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
