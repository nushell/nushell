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

/// Stores the crypto provider used by `rustls`.
///
/// This struct lives in the [`CRYPTO_PROVIDER`] static.  
/// It can't be created manually.
///
/// ## Purpose
///
/// Nushell does **not** use the global `rustls` crypto provider.  
/// You **must** set a provider here—otherwise, any networking command
/// that uses `rustls` won't be able to build a TLS connector.
///
/// This only matters if the **`rustls-tls`** feature is enabled.  
/// Builds with **`native-tls`** ignore this completely.
///
/// ## How to set the provider
///
/// * [`NuCryptoProvider::default`]  
///   Uses a built-in provider that works with official `nu` builds.  
///   This might change in future versions.
///
/// * [`NuCryptoProvider::set`]  
///   Lets you provide your own `CryptoProvider` using a closure:
///
///   ```rust
///   use nu_command::tls::CRYPTO_PROVIDER;
///
///   // Call once at startup
///   CRYPTO_PROVIDER.set(|| Ok(rustls::crypto::ring::default_provider()));
///   ```
///
/// Only the first successful call takes effect. Later calls do nothing and return `false`.
#[derive(Debug)]
pub struct NuCryptoProvider(OnceLock<Result<Arc<CryptoProvider>, ShellError>>);

/// Global [`NuCryptoProvider`] instance.
///
/// When the **`rustls-tls`** feature is active, call
/// [`CRYPTO_PROVIDER.default()`](NuCryptoProvider::default) or  
/// [`CRYPTO_PROVIDER.set(...)`](NuCryptoProvider::set) once at startup
/// to pick the [`CryptoProvider`] that [`rustls`] will use.
///
/// Later TLS code gets the provider using [`get`](NuCryptoProvider::get).  
/// If no provider was set or the closure returned an error, `get` returns a [`ShellError`].
pub static CRYPTO_PROVIDER: NuCryptoProvider = NuCryptoProvider(OnceLock::new());

impl NuCryptoProvider {
    /// Returns the current [`CryptoProvider`].
    ///
    /// Comes from the first call to [`default`](Self::default) or [`set`](Self::set).
    ///
    /// # Errors
    /// - If no provider was set.
    /// - If the `set` closure returned an error.
    pub fn get(&self) -> Result<Arc<CryptoProvider>, ShellError> {
        // we clone here as the Arc for Ok is super cheap and basically all APIs expect an owned
        // ShellError, so we might as well clone here already
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

    /// Sets a custom [`CryptoProvider`].
    ///
    /// Call once at startup, before any TLS code runs.  
    /// The closure runs immediately and the result (either `Ok` or `Err`) is stored.  
    /// Returns whether the provider was stored successfully.
    pub fn set(&self, f: impl FnOnce() -> Result<CryptoProvider, ShellError>) -> bool {
        let value = f().map(Arc::new);
        self.0.set(value).is_ok()
    }

    /// Sets a default [`CryptoProvider`] used in official `nu` builds.
    ///
    /// Should work on most systems, but may not work in every setup.  
    /// If it fails, use [`set`](Self::set) to install a custom one.  
    /// Returns whether the provider was stored successfully.
    pub fn default(&self) -> bool {
        self.set(|| Ok(rustls::crypto::ring::default_provider()))
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
static ROOT_CERT_STORE: LazyLock<Result<Arc<RootCertStore>, ShellError>> = LazyLock::new(|| {
    Ok(Arc::new(rustls::RootCertStore {
        roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
    }))
});

#[doc = include_str!("./tls.rustdoc.md")]
pub fn tls(allow_insecure: bool) -> Result<impl TlsConnector, ShellError> {
    let crypto_provider = CRYPTO_PROVIDER.get()?;

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
