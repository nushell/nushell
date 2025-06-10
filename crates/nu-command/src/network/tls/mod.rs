//! TLS support for networking commands.
//!
//! This module is available when the `network` feature is enabled. It requires
//! either the `native-tls` or `rustls-tls` feature to be selected.
//!
//! See [`tls`] for how to get a TLS connector.

#[cfg(feature = "native-tls")]
#[path = "impl_native_tls.rs"]
mod impl_tls;

#[cfg(feature = "rustls-tls")]
#[path = "impl_rustls.rs"]
mod impl_tls;

#[cfg(all(not(feature = "native-tls"), not(feature = "rustls-tls")))]
compile_error!(
    "No TLS backend enabled. Please enable either the `native-tls` or `rustls-tls` feature."
);

#[cfg(all(feature = "native-tls", feature = "rustls-tls"))]
compile_error!(
    "Multiple TLS backends enabled. Please enable only one of `native-tls` or `rustls-tls`, not both."
);

pub use impl_tls::*;
