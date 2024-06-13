use convert_case::Casing;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use syn::{
    spanned::Spanned, Attribute, Data, DataEnum, DataStruct, DeriveInput, Fields, Generics, Ident,
    Index,
};

use crate::attributes::{self, ContainerAttributes};

#[derive(Debug)]
pub struct IntoValue;
type DeriveError = super::error::DeriveError<IntoValue>;

/// Inner implementation of the `#[derive(IntoValue)]` macro for structs and enums.
///
/// Uses `proc_macro2::TokenStream` for better testing support, unlike `proc_macro::TokenStream`.
///
/// This function directs the `IntoValue` trait derivation to the correct implementation based on
/// the input type:
/// - For structs: [`struct_into_value`]
/// - For enums: [`enum_into_value`]
/// - Unions are not supported and will return an error.
pub fn derive_into_value(input: TokenStream2) -> Result<TokenStream2, DeriveError> {
    let input: DeriveInput = syn::parse2(input).map_err(DeriveError::Syn)?;
    match input.data {
        Data::Struct(data_struct) => Ok(struct_into_value(
            input.ident,
            data_struct,
            input.generics,
            input.attrs,
        )?),
        Data::Enum(data_enum) => Ok(enum_into_value(
            input.ident,
            data_enum,
            input.generics,
            input.attrs,
        )?),
        Data::Union(_) => Err(DeriveError::UnsupportedUnions),
    }
}

/// Implements the `#[derive(IntoValue)]` macro for structs.
///
/// Automatically derives the `IntoValue` trait for any struct where each field implements
/// `IntoValue`.
/// For structs with named fields, the derived implementation creates a `Value::Record` using the
/// struct fields as keys.
/// Each field value is converted using the `IntoValue::into_value` method.
/// For structs with unnamed fields, this generates a `Value::List` with each field in the list.
/// For unit structs, this generates `Value::Nothing`, because there is no data.
///
/// Note: The helper attribute `#[nu_value(...)]` is currently not allowed on structs.
///
/// # Examples
///
/// These examples show what the macro would generate.
///
/// Struct with named fields:
/// ```rust
/// #[derive(IntoValue)]
/// struct Pet {
///     name: String,
///     age: u8,
///     favorite_toy: Option<String>,
/// }
///
/// impl nu_protocol::IntoValue for Pet {
///     fn into_value(self, span: nu_protocol::Span) -> nu_protocol::Value {
///         nu_protocol::Value::record(nu_protocol::record! {
///             "name" => nu_protocol::IntoValue::into_value(self.name, span),
///             "age" => nu_protocol::IntoValue::into_value(self.age, span),
///             "favorite_toy" => nu_protocol::IntoValue::into_value(self.favorite_toy, span),
///         }, span)
///     }
/// }
/// ```
///
/// Struct with unnamed fields:
/// ```rust
/// #[derive(IntoValue)]
/// struct Color(u8, u8, u8);
///
/// impl nu_protocol::IntoValue for Color {
///     fn into_value(self, span: nu_protocol::Span) -> nu_protocol::Value {
///         nu_protocol::Value::list(vec![
///             nu_protocol::IntoValue::into_value(self.0, span),
///             nu_protocol::IntoValue::into_value(self.1, span),
///             nu_protocol::IntoValue::into_value(self.2, span),
///         ], span)
///     }
/// }
/// ```
///
/// Unit struct:
/// ```rust
/// #[derive(IntoValue)]
/// struct Unicorn;
///
/// impl nu_protocol::IntoValue for Unicorn {
///     fn into_value(self, span: nu_protocol::Span) -> nu_protocol::Value {
///         nu_protocol::Value::nothing(span)
///     }
/// }
/// ```
fn struct_into_value(
    ident: Ident,
    data: DataStruct,
    generics: Generics,
    attrs: Vec<Attribute>,
) -> Result<TokenStream2, DeriveError> {
    attributes::deny(&attrs)?;
    attributes::deny_fields(&data.fields)?;
    let record = match &data.fields {
        Fields::Named(fields) => {
            let accessor = fields
                .named
                .iter()
                .map(|field| field.ident.as_ref().expect("named has idents"))
                .map(|ident| quote!(self.#ident));
            fields_return_value(&data.fields, accessor)
        }
        Fields::Unnamed(fields) => {
            let accessor = fields
                .unnamed
                .iter()
                .enumerate()
                .map(|(n, _)| Index::from(n))
                .map(|index| quote!(self.#index));
            fields_return_value(&data.fields, accessor)
        }
        Fields::Unit => quote!(nu_protocol::Value::nothing(span)),
    };
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    Ok(quote! {
        #[automatically_derived]
        impl #impl_generics nu_protocol::IntoValue for #ident #ty_generics #where_clause {
            fn into_value(self, span: nu_protocol::Span) -> nu_protocol::Value {
                #record
            }
        }
    })
}

/// Implements the `#[derive(IntoValue)]` macro for enums.
///
/// This function implements the derive macro `IntoValue` for enums.
/// Currently, only unit enum variants are supported as it is not clear how other types of enums
/// should be represented in a `Value`.
/// For simple enums, we represent the enum as a `Value::String`. For other types of variants, we return an error.
/// The variant name will be case-converted as described by the `#[nu_value(rename_all = "...")]` helper attribute.
/// If no attribute is used, the default is `case_convert::Case::Snake`.
/// The implementation matches over all variants, uses the appropriate variant name, and constructs a `Value::String`.
///
/// This is how such a derived implementation looks:
/// ```rust
/// #[derive(IntoValue)]
/// enum Weather {
///     Sunny,
///     Cloudy,
///     Raining
/// }
///
/// impl nu_protocol::IntoValue for Weather {
///     fn into_value(self, span: nu_protocol::Span) -> nu_protocol::Value {
///         match self {
///             Self::Sunny => nu_protocol::Value::string("sunny", span),
///             Self::Cloudy => nu_protocol::Value::string("cloudy", span),
///             Self::Raining => nu_protocol::Value::string("raining", span),
///         }
///     }
/// }
/// ```
fn enum_into_value(
    ident: Ident,
    data: DataEnum,
    generics: Generics,
    attrs: Vec<Attribute>,
) -> Result<TokenStream2, DeriveError> {
    let container_attrs = ContainerAttributes::parse_attrs(attrs.iter())?;
    let arms: Vec<TokenStream2> = data
        .variants
        .into_iter()
        .map(|variant| {
            attributes::deny(&variant.attrs)?;
            let ident = variant.ident;
            let ident_s = format!("{ident}")
                .as_str()
                .to_case(container_attrs.rename_all);
            match &variant.fields {
                // In the future we can implement more complexe enums here.
                Fields::Named(fields) => Err(DeriveError::UnsupportedEnums {
                    fields_span: fields.span(),
                }),
                Fields::Unnamed(fields) => Err(DeriveError::UnsupportedEnums {
                    fields_span: fields.span(),
                }),
                Fields::Unit => {
                    Ok(quote!(Self::#ident => nu_protocol::Value::string(#ident_s, span)))
                }
            }
        })
        .collect::<Result<_, _>>()?;

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    Ok(quote! {
        impl #impl_generics nu_protocol::IntoValue for #ident #ty_generics #where_clause {
            fn into_value(self, span: nu_protocol::Span) -> nu_protocol::Value {
                match self {
                    #(#arms,)*
                }
            }
        }
    })
}

/// Constructs the final `Value` that the macro generates.
///
/// This function handles the construction of the final `Value` that the macro generates.
/// It is currently only used for structs but may be used for enums in the future.
/// The function takes two parameters: the `fields`, which allow iterating over each field of a data
/// type, and the `accessor`.
/// The fields determine whether we need to generate a `Value::Record`, `Value::List`, or
/// `Value::Nothing`.
/// For named fields, they are also directly used to generate the record key.
///
/// The `accessor` parameter generalizes how the data is accessed.
/// For named fields, this is usually the name of the fields preceded by `self` in a struct, and
/// maybe something else for enums.
/// For unnamed fields, this should be an iterator similar to the one with named fields, but
/// accessing tuple fields, so we get `self.n`.
/// For unit structs, this parameter is ignored.
/// By using the accessor like this, we can have the same code for structs and enums with data
/// variants in the future.
fn fields_return_value(
    fields: &Fields,
    accessor: impl Iterator<Item = impl ToTokens>,
) -> TokenStream2 {
    match fields {
        Fields::Named(fields) => {
            let items: Vec<TokenStream2> = fields
                .named
                .iter()
                .zip(accessor)
                .map(|(field, accessor)| {
                    let ident = field.ident.as_ref().expect("named has idents");
                    let field = ident.to_string();
                    quote!(#field => nu_protocol::IntoValue::into_value(#accessor, span))
                })
                .collect();
            quote! {
                nu_protocol::Value::record(nu_protocol::record! {
                    #(#items),*
                }, span)
            }
        }
        Fields::Unnamed(fields) => {
            let items =
                fields.unnamed.iter().zip(accessor).map(
                    |(_, accessor)| quote!(nu_protocol::IntoValue::into_value(#accessor, span)),
                );
            quote!(nu_protocol::Value::list(std::vec![#(#items),*], span))
        }
        Fields::Unit => quote!(nu_protocol::Value::nothing(span)),
    }
}
