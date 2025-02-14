use std::{any, fmt::Debug, marker::PhantomData};

use proc_macro2::Span;
use proc_macro_error2::{Diagnostic, Level};

#[derive(Debug)]
pub enum DeriveError<M> {
    /// Marker variant, makes the `M` generic parameter valid.
    _Marker(PhantomData<M>),

    /// Parsing errors thrown by `syn`.
    Syn(syn::parse::Error),

    /// `syn::DeriveInput` was a union, currently not supported
    UnsupportedUnions,

    /// Only plain enums are supported right now.
    UnsupportedEnums { fields_span: Span },

    /// Found a `#[nu_value(x)]` attribute where `x` is unexpected.
    UnexpectedAttribute { meta_span: Span },

    /// Found a `#[nu_value(x)]` attribute at a invalid position.
    InvalidAttributePosition { attribute_span: Span },

    /// Found a valid `#[nu_value(x)]` attribute but the passed values is invalid.
    InvalidAttributeValue {
        value_span: Span,
        value: Box<dyn Debug>,
    },

    /// Two keys or variants are called the same name breaking bidirectionality.
    NonUniqueName {
        name: String,
        first: Span,
        second: Span,
    },
}

impl<M> From<syn::parse::Error> for DeriveError<M> {
    fn from(value: syn::parse::Error) -> Self {
        Self::Syn(value)
    }
}

impl<M> From<DeriveError<M>> for Diagnostic {
    fn from(value: DeriveError<M>) -> Self {
        let derive_name = any::type_name::<M>().split("::").last().expect("not empty");
        match value {
            DeriveError::_Marker(_) => panic!("used marker variant"),

            DeriveError::Syn(e) => Diagnostic::spanned(e.span(), Level::Error, e.to_string()),

            DeriveError::UnsupportedUnions => Diagnostic::new(
                Level::Error,
                format!("`{derive_name}` cannot be derived from unions"),
            )
            .help("consider refactoring to a struct".to_string())
            .note("if you really need a union, consider opening an issue on Github".to_string()),

            DeriveError::UnsupportedEnums { fields_span } => Diagnostic::spanned(
                fields_span,
                Level::Error,
                format!("`{derive_name}` can only be derived from plain enums"),
            )
            .help(
                "consider refactoring your data type to a struct with a plain enum as a field"
                    .to_string(),
            )
            .note("more complex enums could be implemented in the future".to_string()),

            DeriveError::InvalidAttributePosition { attribute_span } => Diagnostic::spanned(
                attribute_span,
                Level::Error,
                "invalid attribute position".to_string(),
            )
            .help(format!(
                "check documentation for `{derive_name}` for valid placements"
            )),

            DeriveError::UnexpectedAttribute { meta_span } => {
                Diagnostic::spanned(meta_span, Level::Error, "unknown attribute".to_string()).help(
                    format!("check documentation for `{derive_name}` for valid attributes"),
                )
            }

            DeriveError::InvalidAttributeValue { value_span, value } => {
                Diagnostic::spanned(value_span, Level::Error, format!("invalid value {value:?}"))
                    .help(format!(
                        "check documentation for `{derive_name}` for valid attribute values"
                    ))
            }

            DeriveError::NonUniqueName {
                name,
                first,
                second,
            } => Diagnostic::new(Level::Error, format!("non-unique name {name:?} found"))
                .span_error(first, "first occurrence found here".to_string())
                .span_error(second, "second occurrence found here".to_string())
                .help("use `#[nu_value(rename = \"...\")]` to ensure unique names".to_string()),
        }
    }
}
