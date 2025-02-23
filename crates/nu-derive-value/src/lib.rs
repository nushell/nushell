//! Macro implementations of `#[derive(FromValue, IntoValue)]`.
//!
//! As this crate is a [`proc_macro`] crate, it is only allowed to export
//! [procedural macros](https://doc.rust-lang.org/reference/procedural-macros.html).
//! Therefore, it only exports [`IntoValue`] and [`FromValue`].
//!
//! To get documentation for other functions and types used in this crate, run
//! `cargo doc -p nu-derive-value --document-private-items`.
//!
//! This crate uses a lot of
//! [`proc_macro2::TokenStream`](https://docs.rs/proc-macro2/1.0.24/proc_macro2/struct.TokenStream.html)
//! as `TokenStream2` to allow testing the behavior of the macros directly, including the output
//! token stream or if the macro errors as expected.
//! The tests for functionality can be found in `nu_protocol::value::test_derive`.
//!
//! This documentation is often less reference-heavy than typical Rust documentation.
//! This is because this crate is a dependency for `nu_protocol`, and linking to it would create a
//! cyclic dependency.
//! Also all examples in the documentation aren't tested as this crate cannot be compiled as a
//! normal library very easily.
//! This might change in the future if cargo allows building a proc-macro crate differently for
//! `cfg(doctest)` as they are already doing for `cfg(test)`.
//!
//! The generated code from the derive macros tries to be as
//! [hygienic](https://doc.rust-lang.org/reference/macros-by-example.html#hygiene) as possible.
//! This ensures that the macro can be called anywhere without requiring specific imports.
//! This results in obtuse code, which isn't recommended for manual, handwritten Rust
//! but ensures that no other code may influence this generated code or vice versa.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use proc_macro_error2::{proc_macro_error, Diagnostic};

mod attributes;
mod case;
mod error;
mod from;
mod into;
mod names;
#[cfg(test)]
mod tests;

const HELPER_ATTRIBUTE: &str = "nu_value";

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
