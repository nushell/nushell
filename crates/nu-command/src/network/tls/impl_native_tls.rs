use nu_protocol::ShellError;
use ureq::TlsConnector;

pub fn tls(allow_insecure: bool) -> Result<impl TlsConnector, ShellError> {
    native_tls::TlsConnector::builder()
        .danger_accept_invalid_certs(allow_insecure)
        .build()
        .map_err(|e| ShellError::GenericError {
            error: format!("Failed to build network tls: {}", e),
            msg: String::new(),
            span: None,
            help: None,
            inner: vec![],
        })
}
