use std::sync::{Arc, OnceLock};

use nu_protocol::ShellError;
use rustls::crypto::CryptoProvider;
use ureq::tls::{RootCerts, TlsConfig};

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

#[doc = include_str!("./tls.rustdoc.md")]
pub fn tls(allow_insecure: bool) -> Result<TlsConfig, ShellError> {
    let crypto_provider = CRYPTO_PROVIDER.get()?;
    let config = match allow_insecure {
        false => {
            #[cfg(feature = "os")]
            let certs = RootCerts::PlatformVerifier;

            #[cfg(not(feature = "os"))]
            let certs = RootCerts::WebPki;

            TlsConfig::builder()
                .unversioned_rustls_crypto_provider(crypto_provider)
                .root_certs(certs)
                .build()
        }
        true => TlsConfig::builder().disable_verification(true).build(),
    };

    Ok(config)
}
