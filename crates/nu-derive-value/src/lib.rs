use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use proc_macro_error::proc_macro_error;

mod from;
mod into;

// We use `TokenStream2` internally to allow testing the token streams which
// isn't possible on `proc_macro::TokenStream`.
// Even if not used right now, this will be great in the future.
//
// Also we cannot really document here as `nu_protocol` depends on this crate,
// therefore we cannot depend on `nu_protocol` to make useful links or examples
// here.

/// Derive macro generating an impl of the trait `IntoValue`.
///
/// For further information, see the docs on the trait itself.
#[proc_macro_derive(IntoValue)]
#[proc_macro_error]
pub fn derive_into_value(input: TokenStream) -> TokenStream {
    let input = TokenStream2::from(input);
    let output = match into::derive_into_value(input) {
        Ok(output) => output,
        Err(e) => e.into().abort(),
    };
    TokenStream::from(output)
}

#[proc_macro_derive(FromValue)]
#[proc_macro_error]
pub fn derive_from_value(input: TokenStream) -> TokenStream {
    let input = TokenStream2::from(input);
    let output = match from::derive_from_value(input) {
        Ok(output) => output,
        Err(e) => e.into().abort(),
    };
    TokenStream::from(output)
}
