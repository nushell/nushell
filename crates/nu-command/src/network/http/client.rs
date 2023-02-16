use base64::engine::general_purpose::PAD;
use base64::engine::GeneralPurpose;
use base64::{alphabet, Engine};
use nu_protocol::engine::EngineState;
use nu_protocol::{BufferedReader, PipelineData, RawStream, ShellError, Span, Value};
use reqwest::blocking;
use reqwest::blocking::RequestBuilder;
use std::collections::HashMap;
use std::io::BufReader;

// Only panics if the user agent is invalid but we define it statically so either
// it always or never fails
pub fn http_client(allow_insecure: bool) -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .user_agent("nushell")
        .danger_accept_invalid_certs(allow_insecure)
        .build()
        .expect("Failed to build reqwest client")
}

pub fn response_to_buffer(
    response: blocking::Response,
    engine_state: &EngineState,
    span: Span,
) -> PipelineData {
    // Try to get the size of the file to be downloaded.
    // This is helpful to show the progress of the stream.
    let buffer_size = match &response.headers().get("content-length") {
        Some(content_length) => {
            let content_length = &(*content_length).clone(); // binding

            let content_length = content_length
                .to_str()
                .unwrap_or("")
                .parse::<u64>()
                .unwrap_or(0);

            if content_length == 0 {
                None
            } else {
                Some(content_length)
            }
        }
        _ => None,
    };

    let buffered_input = BufReader::new(response);

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
    mut request: RequestBuilder,
) -> RequestBuilder {
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
        request = request.header("Authorization", format!("Basic {login}"));
    }

    request
}

pub fn request_add_custom_headers(
    headers: Option<Value>,
    mut request: RequestBuilder,
) -> Result<RequestBuilder, ShellError> {
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
                request = request.header(k, s);
            }
        }
    }

    Ok(request)
}
