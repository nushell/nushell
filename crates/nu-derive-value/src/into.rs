use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use syn::{
    spanned::Spanned, Attribute, Data, DataEnum, DataStruct, DeriveInput, Fields, Generics, Ident,
    Index,
};

use crate::{
    attributes::{self, ContainerAttributes, MemberAttributes, ParseAttrs},
    case::Case,
    names::NameResolver,
};

#[derive(Debug)]
pub struct IntoValue;
type DeriveError = super::error::DeriveError<IntoValue>;
type Result<T = TokenStream2> = std::result::Result<T, DeriveError>;

/// Inner implementation of the `#[derive(IntoValue)]` macro for structs and enums.
///
/// Uses `proc_macro2::TokenStream` for better testing support, unlike `proc_macro::TokenStream`.
///
/// This function directs the `IntoValue` trait derivation to the correct implementation based on
/// the input type:
/// - For structs: [`struct_into_value`]
/// - For enums: [`enum_into_value`]
/// - Unions are not supported and will return an error.
pub fn derive_into_value(input: TokenStream2) -> Result {
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
/// The specific keys are resolved by [`NameResolver`](NameResolver::resolve_ident).
/// Each field value is converted using the `IntoValue::into_value` method.
/// For structs with unnamed fields, this generates a `Value::List` with each field in the list.
/// For unit structs, this generates `Value::Nothing`, because there is no data.
///
/// This function provides the signature and prepares the call to the [`fields_return_value`]
/// function which does the heavy lifting of creating the `Value` calls.
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
) -> Result {
    let container_attrs = ContainerAttributes::parse_attrs(attrs.iter())?;
    let record = match &data.fields {
        Fields::Named(fields) => {
            let accessor = fields
                .named
                .iter()
                .map(|field| field.ident.as_ref().expect("named has idents"))
                .map(|ident| quote!(self.#ident));
            fields_return_value(&data.fields, accessor, &container_attrs)?
        }
        Fields::Unnamed(fields) => {
            let accessor = fields
                .unnamed
                .iter()
                .enumerate()
                .map(|(n, _)| Index::from(n))
                .map(|index| quote!(self.#index));
            fields_return_value(&data.fields, accessor, &container_attrs)?
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
/// For simple enums, we represent the enum as a `Value::String`.
/// For other types of variants, we return an error.
///
/// The variant name used in the `Value::String` is resolved by the
/// [`NameResolver`](NameResolver::resolve_ident) with the `default` being [`Case::Snake`].
/// The implementation matches over all variants, uses the appropriate variant name, and constructs
/// a `Value::String`.
///
/// This is how such a derived implementation looks:
/// ```rust
/// #[derive(IntoValue)]
/// enum Weather {
///     Sunny,
///     Cloudy,
///     #[nu_value(rename = "rain")]
///     Raining
/// }
///
/// impl nu_protocol::IntoValue for Weather {
///     fn into_value(self, span: nu_protocol::Span) -> nu_protocol::Value {
///         match self {
///             Self::Sunny => nu_protocol::Value::string("sunny", span),
///             Self::Cloudy => nu_protocol::Value::string("cloudy", span),
///             Self::Raining => nu_protocol::Value::string("rain", span),
///         }
///     }
/// }
/// ```
fn enum_into_value(
    ident: Ident,
    data: DataEnum,
    generics: Generics,
    attrs: Vec<Attribute>,
) -> Result {
    let container_attrs = ContainerAttributes::parse_attrs(attrs.iter())?;
    let mut name_resolver = NameResolver::new();
    let arms: Vec<TokenStream2> = data
        .variants
        .into_iter()
        .map(|variant| {
            let member_attrs = MemberAttributes::parse_attrs(variant.attrs.iter())?;
            let ident = variant.ident;
            let ident_s = name_resolver.resolve_ident(
                &ident,
                &container_attrs,
                &member_attrs,
                Case::Snake,
            )?;
            match &variant.fields {
                // In the future we can implement more complex enums here.
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
        .collect::<Result<_>>()?;

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
/// This function handles the construction of the final `Value` that the macro generates, primarily
/// for structs.
/// It takes three parameters: `fields`, which allows iterating over each field of a data type,
/// `accessor`, which generalizes data access, and `container_attrs`, which is used for the
/// [`NameResolver`].
///
/// - **Field Keys**:
///   The field key is field name of the input struct and resolved the
///   [`NameResolver`](NameResolver::resolve_ident).
///
/// - **Fields Type**:
///   - Determines whether to generate a `Value::Record`, `Value::List`, or `Value::Nothing` based
///     on the nature of the fields.
///   - Named fields are directly used to generate the record key, as described above.
///
/// - **Accessor**:
///   - Generalizes how data is accessed for different data types.
///   - For named fields in structs, this is typically `self.field_name`.
///   - For unnamed fields (e.g., tuple structs), it should be an iterator similar to named fields
///     but accessing fields like `self.0`.
///   - For unit structs, this parameter is ignored.
///
/// This design allows the same function to potentially handle both structs and enums with data
/// variants in the future.
fn fields_return_value(
    fields: &Fields,
    accessor: impl Iterator<Item = impl ToTokens>,
    container_attrs: &ContainerAttributes,
) -> Result {
    match fields {
        Fields::Named(fields) => {
            let mut name_resolver = NameResolver::new();
            let mut items: Vec<TokenStream2> = Vec::with_capacity(fields.named.len());
            for (field, accessor) in fields.named.iter().zip(accessor) {
                let member_attrs = MemberAttributes::parse_attrs(field.attrs.iter())?;
                let ident = field.ident.as_ref().expect("named has idents");
                let field =
                    name_resolver.resolve_ident(ident, container_attrs, &member_attrs, None)?;
                items.push(quote!(#field => nu_protocol::IntoValue::into_value(#accessor, span)));
            }
            Ok(quote! {
                nu_protocol::Value::record(nu_protocol::record! {
                    #(#items),*
                }, span)
            })
        }
        f @ Fields::Unnamed(fields) => {
            attributes::deny_fields(f)?;
            let items =
                fields.unnamed.iter().zip(accessor).map(
                    |(_, accessor)| quote!(nu_protocol::IntoValue::into_value(#accessor, span)),
                );
            Ok(quote!(nu_protocol::Value::list(
                std::vec![#(#items),*],
                span
            )))
        }
        Fields::Unit => Ok(quote!(nu_protocol::Value::nothing(span))),
    }
}
