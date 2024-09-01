use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use syn::{
    ext::IdentExt, spanned::Spanned, Attribute, Data, DataEnum, DataStruct, DeriveInput, Fields,
    Generics, Ident, Type,
};

use crate::{
    attributes::{self, ContainerAttributes},
    case::{Case, Casing},
};

#[derive(Debug)]
pub struct FromValue;
type DeriveError = super::error::DeriveError<FromValue>;

/// Inner implementation of the `#[derive(FromValue)]` macro for structs and enums.
///
/// Uses `proc_macro2::TokenStream` for better testing support, unlike `proc_macro::TokenStream`.
///
/// This function directs the `FromValue` trait derivation to the correct implementation based on
/// the input type:
/// - For structs: [`derive_struct_from_value`]
/// - For enums: [`derive_enum_from_value`]
/// - Unions are not supported and will return an error.
pub fn derive_from_value(input: TokenStream2) -> Result<TokenStream2, DeriveError> {
    let input: DeriveInput = syn::parse2(input).map_err(DeriveError::Syn)?;
    match input.data {
        Data::Struct(data_struct) => Ok(derive_struct_from_value(
            input.ident,
            data_struct,
            input.generics,
            input.attrs,
        )?),
        Data::Enum(data_enum) => Ok(derive_enum_from_value(
            input.ident,
            data_enum,
            input.generics,
            input.attrs,
        )?),
        Data::Union(_) => Err(DeriveError::UnsupportedUnions),
    }
}

/// Implements the `#[derive(FromValue)]` macro for structs.
///
/// This function provides the impl signature for `FromValue`.
/// The implementation for `FromValue::from_value` is handled by [`struct_from_value`] and the
/// `FromValue::expected_type` is handled by [`struct_expected_type`].
fn derive_struct_from_value(
    ident: Ident,
    data: DataStruct,
    generics: Generics,
    attrs: Vec<Attribute>,
) -> Result<TokenStream2, DeriveError> {
    let container_attrs = ContainerAttributes::parse_attrs(attrs.iter())?;
    attributes::deny_fields(&data.fields)?;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let from_value_impl = struct_from_value(&data, container_attrs.rename_all);
    let expected_type_impl =
        struct_expected_type(&data.fields, container_attrs.type_name.as_deref());
    Ok(quote! {
        #[automatically_derived]
        impl #impl_generics nu_protocol::FromValue for #ident #ty_generics #where_clause {
            #from_value_impl
            #expected_type_impl
        }
    })
}

/// Implements `FromValue::from_value` for structs.
///
/// This function constructs the `from_value` function for structs.
/// The implementation is straightforward as most of the heavy lifting is handled by
/// [`parse_value_via_fields`], and this function only needs to construct the signature around it.
///
/// For structs with named fields, this constructs a large return type where each field
/// contains the implementation for that specific field.
/// In structs with unnamed fields, a [`VecDeque`](std::collections::VecDeque) is used to load each
/// field one after another, and the result is used to construct the tuple.
/// For unit structs, this only checks if the input value is `Value::Nothing`.
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
/// impl nu_protocol::FromValue for Pet {
///     fn from_value(
///         v: nu_protocol::Value
///     ) -> std::result::Result<Self, nu_protocol::ShellError> {
///         let span = v.span();
///         let mut record = v.into_record()?;
///         std::result::Result::Ok(Pet {
///             name: <String as nu_protocol::FromValue>::from_value(
///                 record
///                     .remove("name")
///                     .ok_or_else(|| nu_protocol::ShellError::CantFindColumn {
///                         col_name: std::string::ToString::to_string("name"),
///                         span: std::option::Option::None,
///                         src_span: span
///                     })?,
///             )?,
///             age: <u8 as nu_protocol::FromValue>::from_value(
///                 record
///                     .remove("age")
///                     .ok_or_else(|| nu_protocol::ShellError::CantFindColumn {
///                         col_name: std::string::ToString::to_string("age"),
///                         span: std::option::Option::None,
///                         src_span: span
///                     })?,
///             )?,
///             favorite_toy: record
///                 .remove("favorite_toy")
///                 .map(|v| <#ty as nu_protocol::FromValue>::from_value(v))
///                 .transpose()?
///                 .flatten(),
///         })
///     }
/// }
/// ```
///
/// Struct with unnamed fields:
/// ```rust
/// #[derive(IntoValue)]
/// struct Color(u8, u8, u8);
///
/// impl nu_protocol::FromValue for Color {
///     fn from_value(
///         v: nu_protocol::Value
///     ) -> std::result::Result<Self, nu_protocol::ShellError> {
///         let span = v.span();
///         let list = v.into_list()?;
///         let mut deque: std::collections::VecDeque<_> = std::convert::From::from(list);
///         std::result::Result::Ok(Self(
///             {
///                 <u8 as nu_protocol::FromValue>::from_value(
///                     deque
///                         .pop_front()
///                         .ok_or_else(|| nu_protocol::ShellError::CantFindColumn {
///                             col_name: std::string::ToString::to_string(&0),
///                             span: std::option::Option::None,
///                             src_span: span
///                         })?,
///                 )?
///             },
///             {
///                 <u8 as nu_protocol::FromValue>::from_value(
///                     deque
///                         .pop_front()
///                         .ok_or_else(|| nu_protocol::ShellError::CantFindColumn {
///                             col_name: std::string::ToString::to_string(&1),
///                             span: std::option::Option::None,
///                             src_span: span
///                         })?,
///                 )?
///             },
///             {
///                 <u8 as nu_protocol::FromValue>::from_value(
///                     deque
///                         .pop_front()
///                         .ok_or_else(|| nu_protocol::ShellError::CantFindColumn {
///                             col_name: std::string::ToString::to_string(&2),
///                             span: std::option::Option::None,
///                             src_span: span
///                         })?,
///                 )?
///             }
///         ))
///     }
/// }
/// ```
///
/// Unit struct:
/// ```rust
/// #[derive(IntoValue)]
/// struct Unicorn;
///
/// impl nu_protocol::FromValue for Unicorn {
///     fn from_value(
///         v: nu_protocol::Value
///     ) -> std::result::Result<Self, nu_protocol::ShellError> {
///         match v {
///             nu_protocol::Value::Nothing {..} => Ok(Self),
///             v => std::result::Result::Err(nu_protocol::ShellError::CantConvert {
///                 to_type: std::string::ToString::to_string(&<Self as nu_protocol::FromValue>::expected_type()),
///                 from_type: std::string::ToString::to_string(&v.get_type()),
///                 span: v.span(),
///                 help: std::option::Option::None
///             })
///         }
///     }
/// }
/// ```
fn struct_from_value(data: &DataStruct, rename_all: Option<Case>) -> TokenStream2 {
    let body = parse_value_via_fields(&data.fields, quote!(Self), rename_all);
    quote! {
        fn from_value(
            v: nu_protocol::Value
        ) -> std::result::Result<Self, nu_protocol::ShellError> {
            #body
        }
    }
}

/// Implements `FromValue::expected_type` for structs.
///
/// This function constructs the `expected_type` function for structs.
/// The type depends on the `fields`: named fields construct a record type with every key and type
/// laid out.
/// Unnamed fields construct a custom type with the name in the format like
/// `list[type0, type1, type2]`.
/// No fields expect the `Type::Nothing`.
///
/// If `#[nu_value(type_name = "...")]` is used, the output type will be `Type::Custom` with that
/// passed name.
///
/// # Examples
///
/// These examples show what the macro would generate.
///
/// Struct with named fields:
/// ```rust
/// #[derive(FromValue)]
/// struct Pet {
///     name: String,
///     age: u8,
///     favorite_toy: Option<String>,
/// }
///
/// impl nu_protocol::FromValue for Pet {
///     fn expected_type() -> nu_protocol::Type {
///         nu_protocol::Type::Record(
///             std::vec![
///                 (
///                     std::string::ToString::to_string("name"),
///                     <String as nu_protocol::FromValue>::expected_type(),
///                 ),
///                 (
///                     std::string::ToString::to_string("age"),
///                     <u8 as nu_protocol::FromValue>::expected_type(),
///                 ),
///                 (
///                     std::string::ToString::to_string("favorite_toy"),
///                     <Option<String> as nu_protocol::FromValue>::expected_type(),
///                 )
///             ].into_boxed_slice()
///         )
///     }
/// }
/// ```
///
/// Struct with unnamed fields:
/// ```rust
/// #[derive(FromValue)]
/// struct Color(u8, u8, u8);
///
/// impl nu_protocol::FromValue for Color {
///     fn expected_type() -> nu_protocol::Type {
///         nu_protocol::Type::Custom(
///             std::format!(
///                 "[{}, {}, {}]",
///                 <u8 as nu_protocol::FromValue>::expected_type(),
///                 <u8 as nu_protocol::FromValue>::expected_type(),
///                 <u8 as nu_protocol::FromValue>::expected_type()
///             )
///             .into_boxed_str()
///         )
///     }
/// }
/// ```
///
/// Unit struct:
/// ```rust
/// #[derive(FromValue)]
/// struct Unicorn;
///
/// impl nu_protocol::FromValue for Color {
///     fn expected_type() -> nu_protocol::Type {
///         nu_protocol::Type::Nothing
///     }
/// }
/// ```
///
/// Struct with passed type name:
/// ```rust
/// #[derive(FromValue)]
/// #[nu_value(type_name = "bird")]
/// struct Parrot;
///
/// impl nu_protocol::FromValue for Parrot {
///     fn expected_type() -> nu_protocol::Type {
///         nu_protocol::Type::Custom(
///             <std::string::String as std::convert::From::<&str>>::from("bird")
///                 .into_boxed_str()
///         )
///     }
/// }
/// ```
fn struct_expected_type(fields: &Fields, attr_type_name: Option<&str>) -> TokenStream2 {
    let ty = match (fields, attr_type_name) {
        (_, Some(type_name)) => {
            quote!(nu_protocol::Type::Custom(
                <std::string::String as std::convert::From::<&str>>::from(#type_name).into_boxed_str()
            ))
        }
        (Fields::Named(fields), _) => {
            let fields = fields.named.iter().map(|field| {
                let ident = field.ident.as_ref().expect("named has idents");
                let ident_s = ident.unraw().to_string();
                let ty = &field.ty;
                quote! {(
                    std::string::ToString::to_string(#ident_s),
                    <#ty as nu_protocol::FromValue>::expected_type(),
                )}
            });
            quote!(nu_protocol::Type::Record(
                std::vec![#(#fields),*].into_boxed_slice()
            ))
        }
        (Fields::Unnamed(fields), _) => {
            let mut iter = fields.unnamed.iter();
            let fields = fields.unnamed.iter().map(|field| {
                let ty = &field.ty;
                quote!(<#ty as nu_protocol::FromValue>::expected_type())
            });
            let mut template = String::new();
            template.push('[');
            if iter.next().is_some() {
                template.push_str("{}")
            }
            iter.for_each(|_| template.push_str(", {}"));
            template.push(']');
            quote! {
                nu_protocol::Type::Custom(
                    std::format!(
                        #template,
                        #(#fields),*
                    )
                    .into_boxed_str()
                )
            }
        }
        (Fields::Unit, _) => quote!(nu_protocol::Type::Nothing),
    };

    quote! {
        fn expected_type() -> nu_protocol::Type {
            #ty
        }
    }
}

/// Implements the `#[derive(FromValue)]` macro for enums.
///
/// This function constructs the implementation of the `FromValue` trait for enums.
/// It is designed to be on the same level as [`derive_struct_from_value`], even though this
/// implementation is a lot simpler.
/// The main `FromValue::from_value` implementation is handled by [`enum_from_value`].
/// The `FromValue::expected_type` implementation is usually kept empty to use the default
/// implementation, but if `#[nu_value(type_name = "...")]` if given, we use that.
fn derive_enum_from_value(
    ident: Ident,
    data: DataEnum,
    generics: Generics,
    attrs: Vec<Attribute>,
) -> Result<TokenStream2, DeriveError> {
    let container_attrs = ContainerAttributes::parse_attrs(attrs.iter())?;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let from_value_impl = enum_from_value(&data, &attrs)?;
    let expected_type_impl = enum_expected_type(container_attrs.type_name.as_deref());
    Ok(quote! {
        #[automatically_derived]
        impl #impl_generics nu_protocol::FromValue for #ident #ty_generics #where_clause {
            #from_value_impl
            #expected_type_impl
        }
    })
}

/// Implements `FromValue::from_value` for enums.
///
/// This function constructs the `from_value` implementation for enums.
/// It only accepts enums with unit variants, as it is currently unclear how other types of enums
/// should be represented via a `Value`.
/// This function checks that every field is a unit variant and constructs a match statement over
/// all possible variants.
/// The input value is expected to be a `Value::String` containing the name of the variant formatted
/// as defined by the `#[nu_value(rename_all = "...")]` attribute.
/// If no attribute is given, [`snake_case`](heck::ToSnakeCase) is expected.
///
/// If no matching variant is found, `ShellError::CantConvert` is returned.
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
///         let span = v.span();
///         let ty = v.get_type();
///
///         let s = v.into_string()?;
///         match s.as_str() {
///             "sunny" => std::result::Ok(Self::Sunny),
///             "cloudy" => std::result::Ok(Self::Cloudy),
///             "raining" => std::result::Ok(Self::Raining),
///             _ => std::result::Result::Err(nu_protocol::ShellError::CantConvert {
///                 to_type: std::string::ToString::to_string(
///                     &<Self as nu_protocol::FromValue>::expected_type()
///                 ),
///                 from_type: std::string::ToString::to_string(&ty),
///                 span: span,help: std::option::Option::None,
///             }),
///         }
///     }
/// }
/// ```
fn enum_from_value(data: &DataEnum, attrs: &[Attribute]) -> Result<TokenStream2, DeriveError> {
    let container_attrs = ContainerAttributes::parse_attrs(attrs.iter())?;
    let arms: Vec<TokenStream2> = data
        .variants
        .iter()
        .map(|variant| {
            attributes::deny(&variant.attrs)?;
            let ident = &variant.ident;
            let ident_s = format!("{ident}")
                .as_str()
                .to_case(container_attrs.rename_all.unwrap_or(Case::Snake));
            match &variant.fields {
                Fields::Named(fields) => Err(DeriveError::UnsupportedEnums {
                    fields_span: fields.span(),
                }),
                Fields::Unnamed(fields) => Err(DeriveError::UnsupportedEnums {
                    fields_span: fields.span(),
                }),
                Fields::Unit => Ok(quote!(#ident_s => std::result::Result::Ok(Self::#ident))),
            }
        })
        .collect::<Result<_, _>>()?;

    Ok(quote! {
        fn from_value(
            v: nu_protocol::Value
        ) -> std::result::Result<Self, nu_protocol::ShellError> {
            let span = v.span();
            let ty = v.get_type();

            let s = v.into_string()?;
            match s.as_str() {
                #(#arms,)*
                _ => std::result::Result::Err(nu_protocol::ShellError::CantConvert {
                    to_type: std::string::ToString::to_string(
                        &<Self as nu_protocol::FromValue>::expected_type()
                    ),
                    from_type: std::string::ToString::to_string(&ty),
                    span: span,
                    help: std::option::Option::None,
                }),
            }
        }
    })
}

/// Implements `FromValue::expected_type` for enums.
///
/// Since it's difficult to name the type of an enum in the current type system, we want to use the
/// default implementation if `#[nu_value(type_name = "...")]` was *not* given.
/// For that, a `None` value is returned, for a passed type name we return something like this:
/// ```rust
/// #[derive(IntoValue)]
/// #[nu_value(type_name = "sunny | cloudy | raining")]
/// enum Weather {
///     Sunny,
///     Cloudy,
///     Raining
/// }
///
/// impl nu_protocol::FromValue for Weather {
///     fn expected_type() -> nu_protocol::Type {
///         nu_protocol::Type::Custom(
///             <std::string::String as std::convert::From::<&str>>::from("sunny | cloudy | raining")
///                 .into_boxed_str()
///         )
///     }
/// }
/// ```
fn enum_expected_type(attr_type_name: Option<&str>) -> Option<TokenStream2> {
    let type_name = attr_type_name?;
    Some(quote! {
        fn expected_type() -> nu_protocol::Type {
            nu_protocol::Type::Custom(
                <std::string::String as std::convert::From::<&str>>::from(#type_name)
                    .into_boxed_str()
            )
        }
    })
}

/// Parses a `Value` into self.
///
/// This function handles the actual parsing of a `Value` into self.
/// It takes three parameters: `fields`, `self_ident` and `rename_all`.
/// The `fields` parameter determines the expected type of `Value`: named fields expect a
/// `Value::Record`, unnamed fields expect a `Value::List`, and a unit expects `Value::Nothing`.
///
/// For named fields, the `fields` parameter indicates which field in the record corresponds to
/// which struct field.
/// For both named and unnamed fields, it also helps cast the type into a `FromValue` type.
/// This approach maintains
/// [hygiene](https://doc.rust-lang.org/reference/macros-by-example.html#hygiene).
///
/// The `self_ident` parameter is used to describe the identifier of the returned value.
/// For structs, `Self` is usually sufficient, but for enums, `Self::Variant` may be needed in the
/// future.
///
/// The `rename_all` parameter is provided through `#[nu_value(rename_all = "...")]` and describes
/// how, if passed, the field keys in the `Value` should be named.
/// If this is `None`, we keep the names as they are in the struct.
///
/// This function is more complex than the equivalent for `IntoValue` due to error handling
/// requirements.
/// For missing fields, `ShellError::CantFindColumn` is used, and for unit structs,
/// `ShellError::CantConvert` is used.
/// The implementation avoids local variables for fields to prevent accidental shadowing, ensuring
/// that poorly named fields don't cause issues.
/// While this style is not typically recommended in handwritten Rust, it is acceptable for code
/// generation.
fn parse_value_via_fields(
    fields: &Fields,
    self_ident: impl ToTokens,
    rename_all: Option<Case>,
) -> TokenStream2 {
    match fields {
        Fields::Named(fields) => {
            let fields = fields.named.iter().map(|field| {
                let ident = field.ident.as_ref().expect("named has idents");
                let mut ident_s = ident.unraw().to_string();
                if let Some(rename_all) = rename_all {
                    ident_s = ident_s.to_case(rename_all);
                }
                let ty = &field.ty;
                match type_is_option(ty) {
                    true => quote! {
                        #ident: record
                            .remove(#ident_s)
                            .map(|v| <#ty as nu_protocol::FromValue>::from_value(v))
                            .transpose()?
                            .flatten()
                    },

                    false => quote! {
                        #ident: <#ty as nu_protocol::FromValue>::from_value(
                            record
                                .remove(#ident_s)
                                .ok_or_else(|| nu_protocol::ShellError::CantFindColumn {
                                    col_name: std::string::ToString::to_string(#ident_s),
                                    span: std::option::Option::None,
                                    src_span: span
                                })?,
                        )?
                    },
                }
            });
            quote! {
                let span = v.span();
                let mut record = v.into_record()?;
                std::result::Result::Ok(#self_ident {#(#fields),*})
            }
        }
        Fields::Unnamed(fields) => {
            let fields = fields.unnamed.iter().enumerate().map(|(i, field)| {
                let ty = &field.ty;
                quote! {{
                    <#ty as nu_protocol::FromValue>::from_value(
                        deque
                            .pop_front()
                            .ok_or_else(|| nu_protocol::ShellError::CantFindColumn {
                                col_name: std::string::ToString::to_string(&#i),
                                span: std::option::Option::None,
                                src_span: span
                            })?,
                    )?
                }}
            });
            quote! {
                let span = v.span();
                let list = v.into_list()?;
                let mut deque: std::collections::VecDeque<_> = std::convert::From::from(list);
                std::result::Result::Ok(#self_ident(#(#fields),*))
            }
        }
        Fields::Unit => quote! {
            match v {
                nu_protocol::Value::Nothing {..} => Ok(#self_ident),
                v => std::result::Result::Err(nu_protocol::ShellError::CantConvert {
                    to_type: std::string::ToString::to_string(&<Self as nu_protocol::FromValue>::expected_type()),
                    from_type: std::string::ToString::to_string(&v.get_type()),
                    span: v.span(),
                    help: std::option::Option::None
                })
            }
        },
    }
}

const FULLY_QUALIFIED_OPTION: &str = "std::option::Option";
const PARTIALLY_QUALIFIED_OPTION: &str = "option::Option";
const PRELUDE_OPTION: &str = "Option";

/// Check if the field type is an `Option`.
///
/// This function checks if a given type is an `Option`.
/// We assume that an `Option` is [`std::option::Option`] because we can't see the whole code and
/// can't ask the compiler itself.
/// If the `Option` type isn't `std::option::Option`, the user will get a compile error due to a
/// type mismatch.
/// It's very unusual for people to override `Option`, so this should rarely be an issue.
///
/// When [rust#63084](https://github.com/rust-lang/rust/issues/63084) is resolved, we can use
/// [`std::any::type_name`] for a static assertion check to get a more direct error messages.
fn type_is_option(ty: &Type) -> bool {
    let s = ty.to_token_stream().to_string();
    s.starts_with(PRELUDE_OPTION)
        || s.starts_with(PARTIALLY_QUALIFIED_OPTION)
        || s.starts_with(FULLY_QUALIFIED_OPTION)
}
