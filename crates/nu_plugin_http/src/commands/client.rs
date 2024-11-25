use base64::{
    alphabet,
    engine::{general_purpose::PAD, GeneralPurpose},
    Engine,
};
use multipart_rs::MultipartWriter;
use nu_plugin::{EngineInterface, EvaluatedCall};
use nu_protocol::{
    ast::PathMember, record, ByteStream, ByteStreamType, IntoPipelineData, LabeledError,
    PipelineData, ShellError, Signals, Span, Spanned, Type, Value,
};
use serde_json::Value as JsonValue;
use std::{
    collections::HashMap,
    path::PathBuf,
    str::FromStr,
    sync::mpsc::{self, RecvTimeoutError},
    time::Duration,
};
use ureq::{Error, ErrorKind, Request, Response};
use url::Url;

#[derive(Debug, PartialEq, Eq)]
pub enum BodyType {
    Json,
    Form,
    Multipart,
    Unknown(Option<String>),
}

impl From<Option<String>> for BodyType {
    fn from(content_type: Option<String>) -> Self {
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
    engine: &EngineInterface,
) -> Result<ureq::Agent, LabeledError> {
    let tls = native_tls::TlsConnector::builder()
        .danger_accept_invalid_certs(allow_insecure)
        .build()
        .map_err(|e| LabeledError::new(format!("Failed to build network tls: {e}")))?;

    let mut agent_builder = ureq::builder()
        .user_agent("nushell")
        .tls_connector(std::sync::Arc::new(tls));

    if let RedirectMode::Manual | RedirectMode::Error = redirect_mode {
        agent_builder = agent_builder.redirects(0);
    }

    if let Some(http_proxy) = retrieve_http_proxy_from_env(engine)? {
        if let Ok(proxy) = ureq::Proxy::new(http_proxy) {
            agent_builder = agent_builder.proxy(proxy);
        }
    };

    Ok(agent_builder.build())
}

pub fn http_parse_url(span: Span, raw_url: Value) -> Result<(String, Url), ShellError> {
    let requested_url = raw_url.coerce_into_string()?;
    let url = url::Url::parse(&requested_url).map_err(|_| ShellError::InvalidValue {
        valid: "a valid URL".into(),
        actual: requested_url.clone(),
        span,
    })?;
    Ok((requested_url, url))
}

pub fn http_parse_redirect_mode(mode: Option<Spanned<String>>) -> Result<RedirectMode, ShellError> {
    mode.map_or(Ok(RedirectMode::Follow), |v| match v.item.as_str() {
        "follow" | "f" => Ok(RedirectMode::Follow),
        "error" | "e" => Ok(RedirectMode::Error),
        "manual" | "m" => Ok(RedirectMode::Manual),
        s => Err(ShellError::InvalidValue {
            valid: "'follow', 'f', 'error', 'e', 'manual', or 'm'".into(),
            actual: s.into(),
            span: v.span,
        }),
    })
}

pub fn response_to_buffer(
    response: Response,
    engine_state: &EngineInterface,
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

#[allow(clippy::large_enum_variant)]
pub enum LabeledOrRequestError {
    LabeledError(LabeledError),
    RequestError(String, Box<Error>),
}

impl From<LabeledError> for LabeledOrRequestError {
    fn from(error: LabeledError) -> Self {
        LabeledOrRequestError::LabeledError(error)
    }
}

impl From<ShellError> for LabeledOrRequestError {
    fn from(error: ShellError) -> Self {
        LabeledOrRequestError::LabeledError(error.into())
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
    request: Request,
    http_body: HttpBody,
    content_type: Option<String>,
    span: Span,
    signals: &Signals,
) -> Result<Response, LabeledOrRequestError> {
    let request_url = request.url().to_string();

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
                BodyType::Json => send_json_request(&request_url, body, req, span, signals),
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

fn value_to_json_value(v: &Value) -> Result<nu_json::Value, ShellError> {
    let span = v.span();
    Ok(match v {
        Value::Bool { val, .. } => nu_json::Value::Bool(*val),
        Value::Filesize { val, .. } => nu_json::Value::I64(*val),
        Value::Duration { val, .. } => nu_json::Value::I64(*val),
        Value::Date { val, .. } => nu_json::Value::String(val.to_string()),
        Value::Float { val, .. } => nu_json::Value::F64(*val),
        Value::Int { val, .. } => nu_json::Value::I64(*val),
        Value::Nothing { .. } => nu_json::Value::Null,
        Value::String { val, .. } => nu_json::Value::String(val.to_string()),
        Value::Glob { val, .. } => nu_json::Value::String(val.to_string()),
        Value::CellPath { val, .. } => nu_json::Value::Array(
            val.members
                .iter()
                .map(|x| match &x {
                    PathMember::String { val, .. } => nu_json::Value::String(val.clone()),
                    PathMember::Int { val, .. } => nu_json::Value::U64(*val as u64),
                })
                .collect(),
        ),

        Value::List { vals, .. } => nu_json::Value::Array(json_list(vals)?),
        Value::Error { error, .. } => return Err(*error.clone()),
        Value::Closure { .. } | Value::Range { .. } => nu_json::Value::Null,
        Value::Binary { val, .. } => {
            nu_json::Value::Array(val.iter().map(|x| nu_json::Value::U64(*x as u64)).collect())
        }
        Value::Record { val, .. } => {
            let mut m = nu_json::Map::new();
            for (k, v) in &**val {
                m.insert(k.clone(), value_to_json_value(v)?);
            }
            nu_json::Value::Object(m)
        }
        Value::Custom { val, .. } => {
            let collected = val.to_base_value(span)?;
            value_to_json_value(&collected)?
        }
    })
}

fn json_list(input: &[Value]) -> Result<Vec<nu_json::Value>, ShellError> {
    let mut out = vec![];

    for value in input {
        out.push(value_to_json_value(value)?);
    }

    Ok(out)
}

fn send_json_request(
    request_url: &str,
    body: Value,
    req: Request,
    span: Span,
    signals: &Signals,
) -> Result<Response, LabeledOrRequestError> {
    match body {
        Value::Int { .. } | Value::Float { .. } | Value::List { .. } | Value::Record { .. } => {
            let data = value_to_json_value(&body)?;
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
        _ => Err(ShellError::RuntimeTypeMismatch {
            expected: Type::custom("int, float, list, string, or record"),
            actual: body.get_type(),
            span: body.span(),
        })?,
    }
}

fn send_form_request(
    request_url: &str,
    body: Value,
    req: Request,
    span: Span,
    signals: &Signals,
) -> Result<Response, LabeledOrRequestError> {
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
                return Err(LabeledError::new(
                    "Body type 'list' for form requests requires paired values. E.g.: [foo, 10].",
                )
                .with_label("takes lists with an even number of elements", span)
                .with_label("has an odd number of elements", body.span()))?;
            }

            let data = vals
                .chunks(2)
                .map(|it| Ok((it[0].coerce_string()?, it[1].coerce_string()?)))
                .collect::<Result<Vec<_>, ShellError>>()?;

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
        _ => Err(ShellError::RuntimeTypeMismatch {
            expected: Type::custom("list or record"),
            actual: body.get_type(),
            span: body.span(),
        })?,
    }
}

fn send_multipart_request(
    request_url: &str,
    body: Value,
    req: Request,
    span: Span,
    signals: &Signals,
) -> Result<Response, LabeledOrRequestError> {
    let request_fn = match body {
        Value::Record { val, .. } => {
            let mut builder = MultipartWriter::new();

            for (col, val) in val.into_owned() {
                if let Value::Binary { val, .. } = val {
                    let headers = [
                        "Content-Type: application/octet-stream".to_string(),
                        "Content-Transfer-Encoding: binary".to_string(),
                        format!(
                            "Content-Disposition: form-data; name=\"{}\"; filename=\"{}\"",
                            col, col
                        ),
                        format!("Content-Length: {}", val.len()),
                    ];
                    builder
                        .add(val.as_slice(), &headers.join("\r\n"))
                        .expect("reading from a Vec cannot fail");
                } else {
                    let headers = format!(r#"Content-Disposition: form-data; name="{}""#, col);
                    builder
                        .add(val.coerce_into_string()?.as_bytes(), &headers)
                        .expect("reading from a Vec cannot fail");
                }
            }
            builder.finish();

            let (boundary, data) = (builder.boundary, builder.data);
            let content_type = format!("multipart/form-data; boundary={}", boundary);

            move || req.set("Content-Type", &content_type).send_bytes(&data)
        }
        _ => Err(ShellError::RuntimeTypeMismatch {
            expected: Type::record(),
            actual: body.get_type(),
            span: body.span(),
        })?,
    };
    send_cancellable_request(request_url, Box::new(request_fn), span, signals)
}

fn send_default_request(
    request_url: &str,
    body: Value,
    req: Request,
    span: Span,
    signals: &Signals,
) -> Result<Response, LabeledOrRequestError> {
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
        _ => Err(ShellError::RuntimeTypeMismatch {
            expected: Type::custom("binary or string"),
            actual: body.get_type(),
            span: body.span(),
        })?,
    }
}

// Helper method used to make blocking HTTP request calls cancellable with ctrl+c
// ureq functions can block for a long time (default 30s?) while attempting to make an HTTP connection
fn send_cancellable_request(
    request_url: &str,
    request_fn: Box<dyn FnOnce() -> Result<Response, Error> + Sync + Send>,
    span: Span,
    signals: &Signals,
) -> Result<Response, LabeledOrRequestError> {
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
        signals.check(span)?;

        // 100ms wait time chosen arbitrarily
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(result) => {
                return result
                    .map_err(|e| LabeledOrRequestError::RequestError(request_url.into(), e.into()))
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
) -> Result<Response, LabeledOrRequestError> {
    let (tx, rx) = mpsc::channel::<Result<Response, LabeledOrRequestError>>();
    let request_url_string = request_url.to_string();

    // Make the blocking request on a background thread...
    std::thread::Builder::new()
        .name("HTTP requester".to_string())
        .spawn(move || {
            let ret = byte_stream
                .reader()
                .ok_or_else(|| {
                    LabeledOrRequestError::LabeledError(LabeledError::new("Got empty byte stream."))
                })
                .and_then(|reader| {
                    request.send(reader).map_err(|e| {
                        LabeledOrRequestError::RequestError(request_url_string, Box::new(e))
                    })
                });

            // may fail if the user has cancelled the operation
            let _ = tx.send(ret);
        })
        .map_err(ShellError::from)?;

    // ...and poll the channel for responses
    loop {
        signals.check(span)?;

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
        if val < 1 {
            return Err(ShellError::InvalidValue {
                valid: "a positive duration".into(),
                actual: val.to_string(),
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

fn handle_response_error(span: Span, requested_url: &str, response_err: Error) -> LabeledError {
    let msg = match response_err {
        Error::Status(301, _) => "301 resource moved permanently".into(),
        Error::Status(400, _) => "400 bad request".into(),
        Error::Status(403, _) => "403 access forbidden".into(),
        Error::Status(404, _) => "404 not found".into(),
        Error::Status(408, _) => "408 request timed out".into(),
        Error::Status(code, _) => format!("failed with status code {code}"),
        Error::Transport(t) if t.kind() == ErrorKind::ConnectionFailed => {
            format!("failed to establish a connection")
        }
        Error::Transport(t) => t.to_string(),
    };
    LabeledError::new(format!("Request to {requested_url} failed.")).with_label(msg, span)
}

pub struct RequestFlags {
    pub allow_errors: bool,
    pub raw: bool,
    pub full: bool,
}

fn transform_response_using_content_type(
    engine: &EngineInterface,
    span: Span,
    requested_url: &str,
    flags: &RequestFlags,
    resp: Response,
    content_type: &str,
) -> Result<PipelineData, LabeledError> {
    let content_type = mime::Mime::from_str(content_type)
        // there are invalid content types in the wild, so we try to recover
        // Example: `Content-Type: "text/plain"; charset="utf8"` (note the quotes)
        .or_else(|_| mime::Mime::from_str(&content_type.replace('"', "")))
        .unwrap_or(mime::TEXT_PLAIN);

    let ext = match (content_type.type_(), content_type.subtype()) {
        (mime::TEXT, mime::PLAIN) => {
            let path_extension = url::Url::parse(requested_url)
                .map_err(|err| {
                    LabeledError::new(err.to_string())
                        .with_help("cannot parse")
                        .with_label(
                            format!("Cannot parse URL: {requested_url}"),
                            Span::unknown(),
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

    let output = response_to_buffer(resp, engine, span);
    if flags.raw {
        Ok(output)
    } else if let Some(ext) = ext {
        match engine.find_decl(format!("from {ext}"))? {
            Some(id) => Ok(engine.call_decl(id, EvaluatedCall::new(span), output, true, false)?),
            None => Ok(output),
        }
    } else {
        Ok(output)
    }
}

pub fn check_response_redirection(
    redirect_mode: RedirectMode,
    span: Span,
    response: &Result<Response, LabeledOrRequestError>,
) -> Result<(), LabeledError> {
    if let Ok(resp) = response {
        if RedirectMode::Error == redirect_mode && (300..400).contains(&resp.status()) {
            return Err(LabeledError::new(
                "Redirect encountered when redirect handling mode was 'error'.",
            )
            .with_label(
                format!(
                    "failed with status code {} {}",
                    resp.status(),
                    resp.status_text()
                ),
                span,
            ));
        }
    }
    Ok(())
}

fn request_handle_response_content(
    engine: &EngineInterface,
    span: Span,
    requested_url: &str,
    flags: RequestFlags,
    resp: Response,
    request: Request,
) -> Result<PipelineData, LabeledError> {
    // #response_to_buffer moves "resp" making it impossible to read headers later.
    // Wrapping it into a closure to call when needed
    let consume_response_body = |response: Response| {
        let content_type = response.header("content-type").map(|s| s.to_owned());

        match content_type {
            Some(content_type) => transform_response_using_content_type(
                engine,
                span,
                requested_url,
                &flags,
                response,
                &content_type,
            ),
            None => Ok(response_to_buffer(response, engine, span)),
        }
    };

    if flags.full {
        let response_status = resp.status();
        let request_headers_value = headers_to_nu(&extract_request_headers(&request), span);
        let response_headers_value = headers_to_nu(&extract_response_headers(&resp), span);
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
    engine: &EngineInterface,
    span: Span,
    requested_url: &str,
    flags: RequestFlags,
    response: Result<Response, LabeledOrRequestError>,
    request: Request,
) -> Result<PipelineData, LabeledError> {
    match response {
        Ok(resp) => {
            request_handle_response_content(engine, span, requested_url, flags, resp, request)
        }
        Err(e) => match e {
            LabeledOrRequestError::LabeledError(e) => Err(e),
            LabeledOrRequestError::RequestError(_, e) => {
                if flags.allow_errors {
                    if let Error::Status(_, resp) = *e {
                        Ok(request_handle_response_content(
                            engine,
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

fn headers_to_nu(headers: &Headers, span: Span) -> Value {
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

    Value::list(vals, span)
}

pub fn request_handle_response_headers(
    span: Span,
    response: Result<Response, LabeledOrRequestError>,
) -> Result<Value, LabeledError> {
    match response {
        Ok(resp) => Ok(headers_to_nu(&extract_response_headers(&resp), span)),
        Err(e) => match e {
            LabeledOrRequestError::LabeledError(e) => Err(e),
            LabeledOrRequestError::RequestError(requested_url, e) => {
                Err(handle_response_error(span, &requested_url, *e))
            }
        },
    }
}

fn retrieve_http_proxy_from_env(engine: &EngineInterface) -> Result<Option<String>, ShellError> {
    engine
        .get_env_var("http_proxy")?
        .or(engine.get_env_var("HTTP_PROXY")?)
        .or(engine.get_env_var("https_proxy")?)
        .or(engine.get_env_var("HTTPS_PROXY")?)
        .or(engine.get_env_var("ALL_PROXY")?)
        .map(Value::coerce_into_string)
        .transpose()
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
