use std::{any, fmt::Display, marker::PhantomData};

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use proc_macro_error::{proc_macro_error, Diagnostic, Level};

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

/// Derive macro generating an impl of the trait `FromValue`.
///
/// For further information, see the docs on the trait itself.
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

enum DeriveError<M> {
    _Marker(PhantomData<M>),
    Syn(syn::parse::Error),
    UnsupportedUnions,
    UnsupportedEnums { fields_span: Span },
}

impl<M> From<DeriveError<M>> for Diagnostic {
    fn from(value: DeriveError<M>) -> Self {
        let derive_name = any::type_name::<M>().split("::").last().expect("not empty");
        match value {
            DeriveError::_Marker(_) => panic!("used marker variant"),
            DeriveError::Syn(e) => Diagnostic::spanned(e.span(), Level::Error, e.to_string()),
            DeriveError::UnsupportedUnions => Diagnostic::new(
                Level::Error,
                format!("`{}` cannot be derived from unions", derive_name),
            )
            .help("consider refactoring to a struct".to_string())
            .note("if you really need a union, consider opening an issue on Github".to_string()),
            DeriveError::UnsupportedEnums { fields_span } => Diagnostic::spanned(
                fields_span,
                Level::Error,
                format!("`{}` can only be derived from plain enums", derive_name),
            )
            .help(
                "consider refactoring your data type to a struct with a plain enum as a field"
                    .to_string(),
            )
            .note("more complex enums could be implemented in the future".to_string()),
        }
    }
}
