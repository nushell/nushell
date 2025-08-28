Provide a [`TlsConfig`] for [`ureq`].

This is used by Nushell's networking commands (`http`) to handle secure 
(or optionally insecure) HTTP connections.
The returned connector enables `ureq` to perform HTTPS requests. 
If `allow_insecure` is set to `true`, certificate verification is disabled.

This function is only available when the `network` feature is enabled,
and requires exactly one of the `native-tls` or `rustls-tls` features to 
be active.

# With `native-tls`

When built with `native-tls`, this uses the platform TLS backend:
- OpenSSL on most Unix systems
- SChannel on Windows

These are mature and widely-deployed TLS implementations. 
Expect strong platform integration.

# With `rustls-tls`

When built with `rustls-tls`, this uses the pure-Rust [`rustls`] library for TLS.
This has several benefits:
- Easier cross-compilation (no need for OpenSSL headers or linker setup)
- Works with `musl` targets out of the box
- Can be compiled to WASM

A [`NuCryptoProvider`] must be configured before calling this function. 
Use [`CRYPTO_PROVIDER.default()`](NuCryptoProvider::default) or 
[`CRYPTO_PROVIDER.set(...)`](NuCryptoProvider::set) to initialize it.