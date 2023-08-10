use crate::formats::value_to_json_value;
use base64::engine::general_purpose::PAD;
use base64::engine::GeneralPurpose;
use base64::{alphabet, Engine};
use nu_protocol::ast::Call;
use nu_protocol::engine::{EngineState, Stack};
use nu_protocol::{
    BufferedReader, IntoPipelineData, PipelineData, RawStream, ShellError, Span, Value,
};

use ureq::{Error, ErrorKind, Request, Response};

use std::collections::HashMap;
use std::io::BufReader;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::Arc;
use std::time::Duration;
use url::Url;

#[derive(PartialEq, Eq)]
pub enum BodyType {
    Json,
    Form,
    Unknown,
}

// Only panics if the user agent is invalid but we define it statically so either
// it always or never fails
#[cfg(all(feature = "rustls", not(feature = "native-tls")))]
struct NoCertificateVerification {}

#[cfg(all(feature = "rustls", not(feature = "native-tls")))]
impl rustls::client::ServerCertVerifier for NoCertificateVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::Certificate,
        _intermediates: &[rustls::Certificate],
        _server_name: &rustls::ServerName,
        _scts: &mut dyn Iterator<Item = &[u8]>,
        _ocsp: &[u8],
        _now: std::time::SystemTime,
    ) -> std::result::Result<rustls::client::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::ServerCertVerified::assertion())
    }
}

#[cfg(all(feature = "rustls", not(feature = "native-tls")))]
pub fn http_client(allow_insecure: bool) -> ureq::Agent {
    if allow_insecure {
        let mut root_store = rustls::RootCertStore::empty();
        root_store.add_trust_anchors(webpki_roots::TLS_SERVER_ROOTS.iter().map(|ta| {
            rustls::OwnedTrustAnchor::from_subject_spki_name_constraints(
                ta.subject,
                ta.spki,
                ta.name_constraints,
            )
        }));

        let mut config = rustls::ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(root_store)
            .with_no_client_auth();
        config
            .dangerous()
            .set_certificate_verifier(Arc::new(NoCertificateVerification {}));
        ureq::builder()
            .tls_config(Arc::new(config))
            .user_agent("nushell")
            .build()
    } else {
        ureq::builder().user_agent("nushell").build()
    }
}

#[cfg(all(not(feature = "rustls"), feature = "native-tls"))]
pub fn http_client(allow_insecure: bool) -> ureq::Agent {
    let tls = native_tls::TlsConnector::builder()
        .danger_accept_invalid_certs(allow_insecure)
        .build()
        .expect("Failed to build network tls");

    ureq::builder()
        .user_agent("nushell")
        .tls_connector(std::sync::Arc::new(tls))
        .build()
}

#[cfg(all(not(feature = "rustls"), not(feature = "native-tls")))]
compile_error!("Either feature native_tls or rustls features must be specified, none supplied.");

#[cfg(all(feature = "rustls", feature = "native-tls"))]
compile_error!("Either feature native_tls or rustls features must be specified, both supplied, --no-default-features might be needed.");

pub fn http_parse_url(
    call: &Call,
    span: Span,
    raw_url: Value,
) -> Result<(String, Url), ShellError> {
    let requested_url = raw_url.as_string()?;
    let url = match url::Url::parse(&requested_url) {
        Ok(u) => u,
        Err(_e) => {
            return Err(ShellError::UnsupportedInput(
                "Incomplete or incorrect URL. Expected a full URL, e.g., https://www.example.com"
                    .to_string(),
                format!("value: '{requested_url:?}'"),
                call.head,
                span,
            ));
        }
    };

    Ok((requested_url, url))
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
            Box::new(BufferedReader {
                input: buffered_input,
            }),
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
        Value::Record { cols, vals, .. } if body_type == BodyType::Form => {
            let mut data: Vec<(String, String)> = Vec::with_capacity(cols.len());

            for (col, val) in cols.iter().zip(vals.iter()) {
                let val_string = val.as_string()?;
                data.push((col.clone(), val_string))
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
                return Err(ShellErrorOrRequestError::ShellError(ShellError::IOError(
                    "unsupported body input".into(),
                )));
            }

            let data = vals
                .chunks(2)
                .map(|it| Ok((it[0].as_string()?, it[1].as_string()?)))
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
        _ => Err(ShellErrorOrRequestError::ShellError(ShellError::IOError(
            "unsupported body input".into(),
        ))),
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
        .expect("Failed to create thread");

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
                err_message: "Timeout value must be an integer and larger than 0".to_string(),
                span: timeout.expect_span(),
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
            Value::Record { cols, vals, .. } => {
                for (k, v) in cols.iter().zip(vals.iter()) {
                    custom_headers.insert(k.to_string(), v.clone());
                }
            }

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
                            return Err(ShellError::CantConvert {
                                to_type: "string list or single row".into(),
                                from_type: x.get_type().to_string(),
                                span: headers.span().unwrap_or_else(|_| Span::new(0, 0)),
                                help: None,
                            });
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
                return Err(ShellError::CantConvert {
                    to_type: "string list or single row".into(),
                    from_type: x.get_type().to_string(),
                    span: headers.span().unwrap_or_else(|_| Span::new(0, 0)),
                    help: None,
                });
            }
        };

        for (k, v) in &custom_headers {
            if let Ok(s) = v.as_string() {
                request = request.set(k, &s);
            }
        }
    }

    Ok(request)
}

fn handle_response_error(span: Span, requested_url: &str, response_err: Error) -> ShellError {
    match response_err {
        Error::Status(301, _) => ShellError::NetworkFailure(
            format!("Resource moved permanently (301): {requested_url:?}"),
            span,
        ),
        Error::Status(400, _) => {
            ShellError::NetworkFailure(format!("Bad request (400) to {requested_url:?}"), span)
        }
        Error::Status(403, _) => {
            ShellError::NetworkFailure(format!("Access forbidden (403) to {requested_url:?}"), span)
        }
        Error::Status(404, _) => ShellError::NetworkFailure(
            format!("Requested file not found (404): {requested_url:?}"),
            span,
        ),
        Error::Status(408, _) => {
            ShellError::NetworkFailure(format!("Request timeout (408): {requested_url:?}"), span)
        }
        Error::Status(_, _) => ShellError::NetworkFailure(
            format!(
                "Cannot make request to {:?}. Error is {:?}",
                requested_url,
                response_err.to_string()
            ),
            span,
        ),

        Error::Transport(t) => match t {
            t if t.kind() == ErrorKind::ConnectionFailed => ShellError::NetworkFailure(
                format!("Cannot make request to {requested_url}, there was an error establishing a connection.",),
                span,
            ),
            t => ShellError::NetworkFailure(t.to_string(), span),
        },
    }
}

pub struct RequestFlags {
    pub allow_errors: bool,
    pub raw: bool,
    pub full: bool,
}

#[allow(clippy::needless_return)]
fn transform_response_using_content_type(
    engine_state: &EngineState,
    stack: &mut Stack,
    span: Span,
    requested_url: &str,
    flags: &RequestFlags,
    resp: Response,
    content_type: &str,
) -> Result<PipelineData, ShellError> {
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
            let path_extension = url::Url::parse(requested_url)
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
    if flags.raw {
        return Ok(output);
    } else if let Some(ext) = ext {
        return match engine_state.find_decl(format!("from {ext}").as_bytes(), &[]) {
            Some(converter_id) => engine_state.get_decl(converter_id).run(
                engine_state,
                stack,
                &Call::new(span),
                output,
            ),
            None => Ok(output),
        };
    } else {
        return Ok(output);
    };
}

fn request_handle_response_content(
    engine_state: &EngineState,
    stack: &mut Stack,
    span: Span,
    requested_url: &str,
    flags: RequestFlags,
    resp: Response,
) -> Result<PipelineData, ShellError> {
    let response_headers: Option<PipelineData> = if flags.full {
        let headers_raw = request_handle_response_headers_raw(span, &resp)?;
        Some(headers_raw)
    } else {
        None
    };

    let response_status = resp.status();
    let content_type = resp.header("content-type").map(|s| s.to_owned());
    let formatted_content = match content_type {
        Some(content_type) => transform_response_using_content_type(
            engine_state,
            stack,
            span,
            requested_url,
            &flags,
            resp,
            &content_type,
        ),
        None => Ok(response_to_buffer(resp, engine_state, span)),
    };
    if flags.full {
        let full_response = Value::Record {
            cols: vec![
                "headers".to_string(),
                "body".to_string(),
                "status".to_string(),
            ],
            vals: vec![
                match response_headers {
                    Some(headers) => headers.into_value(span),
                    None => Value::nothing(span),
                },
                formatted_content?.into_value(span),
                Value::int(response_status as i64, span),
            ],
            span,
        }
        .into_pipeline_data();
        Ok(full_response)
    } else {
        Ok(formatted_content?)
    }
}

pub fn request_handle_response(
    engine_state: &EngineState,
    stack: &mut Stack,
    span: Span,
    requested_url: &str,
    flags: RequestFlags,
    response: Result<Response, ShellErrorOrRequestError>,
) -> Result<PipelineData, ShellError> {
    match response {
        Ok(resp) => {
            request_handle_response_content(engine_state, stack, span, requested_url, flags, resp)
        }
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

pub fn request_handle_response_headers_raw(
    span: Span,
    response: &Response,
) -> Result<PipelineData, ShellError> {
    let header_names = response.headers_names();

    let cols = vec!["name".to_string(), "value".to_string()];
    let mut vals = Vec::with_capacity(header_names.len());

    for name in &header_names {
        let is_duplicate = vals.iter().any(|val| {
            if let Value::Record { vals, .. } = val {
                if let Some(Value::String {
                    val: header_name, ..
                }) = vals.get(0)
                {
                    return name == header_name;
                }
            }
            false
        });
        if !is_duplicate {
            // Use the ureq `Response.all` api to get all of the header values with a given name.
            // This interface is why we needed to check if we've already parsed this header name.
            for str_value in response.all(name) {
                let header = vec![Value::string(name, span), Value::string(str_value, span)];
                vals.push(Value::record(cols.clone(), header, span));
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
        Ok(resp) => request_handle_response_headers_raw(span, &resp),
        Err(e) => match e {
            ShellErrorOrRequestError::ShellError(e) => Err(e),
            ShellErrorOrRequestError::RequestError(requested_url, e) => {
                Err(handle_response_error(span, &requested_url, *e))
            }
        },
    }
}
