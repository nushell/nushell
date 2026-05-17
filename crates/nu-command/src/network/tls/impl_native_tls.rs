use nu_protocol::ShellError;
use ureq::tls::{TlsConfig, TlsProvider};

#[doc = include_str!("./tls_config.rustdoc.md")]
pub fn tls_config(allow_insecure: bool) -> Result<TlsConfig, ShellError> {
    // The impl for rustls has the option to use other root certificates.
    // This is kind ob unnecessary for the native tls, as we expect to run with an OS.
    Ok(TlsConfig::builder()
        .provider(TlsProvider::NativeTls)
        .disable_verification(allow_insecure)
        .build())
}
