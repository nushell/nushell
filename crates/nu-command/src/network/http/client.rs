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

pub fn send_request(
    request: Request,
    span: Span,
    body: Option<Value>,
    content_type: Option<String>,
) -> Result<Response, ShellError> {
    let request_url = request.url().to_string();
    if body.is_none() {
        return request
            .call()
            .map_err(|err| handle_response_error(span, &request_url, err));
    }
    let body = body.expect("Should never be none.");

    let body_type = match content_type {
        Some(it) if it == "application/json" => BodyType::Json,
        Some(it) if it == "application/x-www-form-urlencoded" => BodyType::Form,
        _ => BodyType::Unknown,
    };
    match body {
        Value::Binary { val, .. } => request
            .send_bytes(&val)
            .map_err(|err| handle_response_error(span, &request_url, err)),
        Value::String { val, .. } => request
            .send_string(&val)
            .map_err(|err| handle_response_error(span, &request_url, err)),
        Value::Record { .. } if body_type == BodyType::Json => {
            let data = value_to_json_value(&body)?;
            request
                .send_json(data)
                .map_err(|err| handle_response_error(span, &request_url, err))
        }
        Value::Record { cols, vals, .. } if body_type == BodyType::Form => {
            let mut data: Vec<(String, String)> = Vec::with_capacity(cols.len());

            for (col, val) in cols.iter().zip(vals.iter()) {
                data.push((col.clone(), val.as_string()?))
            }

            let data = data
                .iter()
                .map(|(a, b)| (a.as_str(), b.as_str()))
                .collect::<Vec<(&str, &str)>>();

            request
                .send_form(&data[..])
                .map_err(|err| handle_response_error(span, &request_url, err))
        }
        Value::List { vals, .. } if body_type == BodyType::Form => {
            if vals.len() % 2 != 0 {
                return Err(ShellError::IOError("unsupported body input".into()));
            }

            let data = vals
                .chunks(2)
                .map(|it| Ok((it[0].as_string()?, it[1].as_string()?)))
                .collect::<Result<Vec<(String, String)>, ShellError>>()?;

            let data = data
                .iter()
                .map(|(a, b)| (a.as_str(), b.as_str()))
                .collect::<Vec<(&str, &str)>>();

            request
                .send_form(&data)
                .map_err(|err| handle_response_error(span, &request_url, err))
        }
        _ => Err(ShellError::IOError("unsupported body input".into())),
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

pub fn request_handle_response(
    engine_state: &EngineState,
    stack: &mut Stack,
    span: Span,
    requested_url: &String,
    raw: bool,
    response: Result<Response, ShellError>,
) -> Result<PipelineData, ShellError> {
    match response {
        Ok(resp) => match resp.header("content-type") {
            Some(content_type) => {
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
        Err(e) => Err(e),
    }
}

pub fn request_handle_response_headers(
    span: Span,
    response: Result<Response, ShellError>,
) -> Result<PipelineData, ShellError> {
    match response {
        Ok(resp) => {
            let cols = resp.headers_names();

            let mut vals = Vec::with_capacity(cols.len());
            for key in &cols {
                match resp.header(key) {
                    // match value.to_str() {
                    Some(str_value) => vals.push(Value::String {
                        val: str_value.to_string(),
                        span,
                    }),
                    None => {
                        return Err(ShellError::GenericError(
                            "Failure when converting header value".to_string(),
                            "".to_string(),
                            None,
                            Some("Failure when converting header value".to_string()),
                            Vec::new(),
                        ))
                    }
                }
            }

            Ok(Value::Record { cols, vals, span }.into_pipeline_data())
        }
        Err(e) => Err(e),
    }
}
