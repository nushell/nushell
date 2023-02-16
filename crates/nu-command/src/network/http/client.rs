use nu_protocol::engine::EngineState;
use nu_protocol::{BufferedReader, PipelineData, RawStream, Span};
use reqwest::blocking;
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
