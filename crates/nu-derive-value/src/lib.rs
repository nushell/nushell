use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use proc_macro_error::{proc_macro_error, Diagnostic};

mod attributes;
mod error;
mod from;
mod into;
#[cfg(test)]
mod tests;

const HELPER_ATTRIBUTE: &str = "nu_value";

// We use `TokenStream2` internally to allow testing the token streams which
// isn't possible on `proc_macro::TokenStream`.
//
// We cannot really document here as `nu_protocol` depends on this crate,
// therefore we cannot depend on `nu_protocol` to make useful links or examples
// here.

/// Derive macro generating an impl of the trait `IntoValue`.
///
/// For further information, see the docs on the trait itself.
#[proc_macro_derive(IntoValue, attributes(nu_value))]
#[proc_macro_error]
pub fn derive_into_value(input: TokenStream) -> TokenStream {
    let input = TokenStream2::from(input);
    let output = match into::derive_into_value(input) {
        Ok(output) => output,
        Err(e) => Diagnostic::from(e).abort(),
    };
    TokenStream::from(output)
}

/// Derive macro generating an impl of the trait `FromValue`.
///
/// For further information, see the docs on the trait itself.
#[proc_macro_derive(FromValue, attributes(nu_value))]
#[proc_macro_error]
pub fn derive_from_value(input: TokenStream) -> TokenStream {
    let input = TokenStream2::from(input);
    let output = match from::derive_from_value(input) {
        Ok(output) => output,
        Err(e) => Diagnostic::from(e).abort(),
    };
    TokenStream::from(output)
}
