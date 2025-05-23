use std::{
    ops::Deref,
    sync::{Arc, LazyLock, OnceLock},
};

use nu_engine::command_prelude::IoError;
use nu_protocol::ShellError;
use rustls::{
    DigitallySignedStruct, RootCertStore, SignatureScheme,
    client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
    crypto::CryptoProvider,
    pki_types::{CertificateDer, ServerName, UnixTime},
};
use ureq::TlsConnector;

// TODO: replace all these generic errors with proper errors

#[derive(Debug)]
pub struct NuCryptoProvider(OnceLock<Result<Arc<CryptoProvider>, ShellError>>);
pub static CRYPTO_PROVIDER: NuCryptoProvider = NuCryptoProvider(OnceLock::new());

impl NuCryptoProvider {
    pub fn get(&self) -> Result<Arc<CryptoProvider>, ShellError> {
        // we clone here as the Arc for ok is super cheap and basically all apis expect an owned
        // shell error, so we might as well clone here already
        match self.0.get() {
            Some(val) => val.clone(),
            None => Err(ShellError::GenericError {
                error: "tls crypto provider not found".to_string(),
                msg: "no crypto provider for rustls was defined".to_string(),
                span: None,
                help: Some("ensure that nu_command::tls::CRYPTO_PROVIDER is set".to_string()),
                inner: vec![],
            }),
        }
    }

    pub fn set(&self, f: impl FnOnce() -> Result<CryptoProvider, ShellError>) -> bool {
        let value = f().map(|v| Arc::new(v));
        match self.0.set(value) {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    pub fn default(&self) -> bool {
        self.set(|| Ok(rustls::crypto::aws_lc_rs::default_provider()))
    }
}

#[cfg(feature = "os")]
static ROOT_CERT_STORE: LazyLock<Result<Arc<RootCertStore>, ShellError>> = LazyLock::new(|| {
    let mut roots = RootCertStore::empty();

    let native_certs = rustls_native_certs::load_native_certs();

    let errors: Vec<_> = native_certs
        .errors
        .into_iter()
        .map(|err| match err.kind {
            rustls_native_certs::ErrorKind::Io { inner, path } => ShellError::Io(
                IoError::new_internal_with_path(inner, err.context, nu_protocol::location!(), path),
            ),
            rustls_native_certs::ErrorKind::Os(error) => ShellError::GenericError {
                error: error.to_string(),
                msg: err.context.to_string(),
                span: None,
                help: None,
                inner: vec![],
            },
            rustls_native_certs::ErrorKind::Pem(error) => ShellError::GenericError {
                error: error.to_string(),
                msg: err.context.to_string(),
                span: None,
                help: None,
                inner: vec![],
            },
            _ => ShellError::GenericError {
                error: String::from("unknown error loading native certs"),
                msg: err.context.to_string(),
                span: None,
                help: None,
                inner: vec![],
            },
        })
        .collect();
    if !errors.is_empty() {
        return Err(ShellError::GenericError {
            error: String::from("error loading native certs"),
            msg: String::from("could not load native certs"),
            span: None,
            help: None,
            inner: errors,
        });
    }

    for cert in native_certs.certs {
        roots.add(cert).map_err(|err| ShellError::GenericError {
            error: err.to_string(),
            msg: String::from("could not add root cert"),
            span: None,
            help: None,
            inner: vec![],
        })?;
    }

    Ok(Arc::new(roots))
});

#[cfg(not(feature = "os"))]
static ROOT_CERT_STORE: LazyLock<Result<Arc<RootCertStore>, ShellError>> = todo!();

pub fn tls(allow_insecure: bool) -> Result<impl TlsConnector, ShellError> {
    let crypto_provider = dbg!(CRYPTO_PROVIDER.get()?);

    let make_protocol_versions_error = |err: rustls::Error| ShellError::GenericError {
        error: err.to_string(),
        msg: "crypto provider is incompatible with protocol versions".to_string(),
        span: None,
        help: None,
        inner: vec![],
    };

    let client_config = match allow_insecure {
        false => rustls::ClientConfig::builder_with_provider(crypto_provider)
            .with_safe_default_protocol_versions()
            .map_err(make_protocol_versions_error)?
            .with_root_certificates(ROOT_CERT_STORE.deref().clone()?)
            .with_no_client_auth(),
        true => rustls::ClientConfig::builder_with_provider(crypto_provider)
            .with_safe_default_protocol_versions()
            .map_err(make_protocol_versions_error)?
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(UnsecureServerCertVerifier))
            .with_no_client_auth(),
    };

    Ok(Arc::new(client_config))
}

#[derive(Debug)]
struct UnsecureServerCertVerifier;

impl ServerCertVerifier for UnsecureServerCertVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        vec![
            SignatureScheme::RSA_PKCS1_SHA1,
            SignatureScheme::ECDSA_SHA1_Legacy,
            SignatureScheme::RSA_PKCS1_SHA256,
            SignatureScheme::ECDSA_NISTP256_SHA256,
            SignatureScheme::RSA_PKCS1_SHA384,
            SignatureScheme::ECDSA_NISTP384_SHA384,
            SignatureScheme::RSA_PKCS1_SHA512,
            SignatureScheme::ECDSA_NISTP521_SHA512,
            SignatureScheme::RSA_PSS_SHA256,
            SignatureScheme::RSA_PSS_SHA384,
            SignatureScheme::RSA_PSS_SHA512,
            SignatureScheme::ED25519,
            SignatureScheme::ED448,
        ]
    }
}
