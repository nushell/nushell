use proc_macro2::TokenStream as TokenStream2;
use quote::{ToTokens, quote};
use syn::{
    Attribute, Data, DataEnum, DataStruct, DeriveInput, Fields, Generics, Ident, Type,
    spanned::Spanned,
};

use crate::{
    attributes::{self, ContainerAttributes, MemberAttributes, ParseAttrs},
    case::Case,
    names::NameResolver,
};

#[derive(Debug)]
pub struct FromValue;
type DeriveError = super::error::DeriveError<FromValue>;
type Result<T = TokenStream2> = std::result::Result<T, DeriveError>;

/// Inner implementation of the `#[derive(FromValue)]` macro for structs and enums.
///
/// Uses `proc_macro2::TokenStream` for better testing support, unlike `proc_macro::TokenStream`.
///
/// This function directs the `FromValue` trait derivation to the correct implementation based on
/// the input type:
/// - For structs: [`derive_struct_from_value`]
/// - For enums: [`derive_enum_from_value`]
/// - Unions are not supported and will return an error.
pub fn derive_from_value(input: TokenStream2) -> Result {
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
) -> Result {
    let container_attrs = ContainerAttributes::parse_attrs(attrs.iter())?;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let from_value_impl = struct_from_value(&data, &container_attrs)?;
    let expected_type_impl = struct_expected_type(
        &data.fields,
        container_attrs.type_name.as_deref(),
        &container_attrs,
    )?;
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
fn struct_from_value(data: &DataStruct, container_attrs: &ContainerAttributes) -> Result {
    let body = parse_value_via_fields(&data.fields, quote!(Self), container_attrs)?;
    Ok(quote! {
        fn from_value(
            v: nu_protocol::Value
        ) -> std::result::Result<Self, nu_protocol::ShellError> {
            #body
        }
    })
}

/// Implements `FromValue::expected_type` for structs.
///
/// This function constructs the `expected_type` function for structs based on the provided fields.
/// The type depends on the `fields`:
/// - Named fields construct a record type where each key corresponds to a field name.
///   The specific keys are resolved by [`NameResolver::resolve_ident`].
/// - Unnamed fields construct a custom type with the format `list[type0, type1, type2]`.
/// - Unit structs expect `Type::Nothing`.
///
/// If the `#[nu_value(type_name = "...")]` attribute is used, the output type will be
/// `Type::Custom` with the provided name.
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
///     #[nu_value(rename = "toy")]
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
///                     std::string::ToString::to_string("toy"),
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
fn struct_expected_type(
    fields: &Fields,
    attr_type_name: Option<&str>,
    container_attrs: &ContainerAttributes,
) -> Result {
    let ty = match (fields, attr_type_name) {
        (_, Some(type_name)) => {
            quote!(nu_protocol::Type::Custom(
                <std::string::String as std::convert::From::<&str>>::from(#type_name).into_boxed_str()
            ))
        }
        (Fields::Named(fields), _) => {
            let mut name_resolver = NameResolver::new();
            let mut fields_ts = Vec::with_capacity(fields.named.len());
            for field in fields.named.iter() {
                let member_attrs = MemberAttributes::parse_attrs(&field.attrs)?;
                let ident = field.ident.as_ref().expect("named has idents");
                let ident_s =
                    name_resolver.resolve_ident(ident, container_attrs, &member_attrs, None)?;
                let ty = &field.ty;
                fields_ts.push(quote! {(
                    std::string::ToString::to_string(#ident_s),
                    <#ty as nu_protocol::FromValue>::expected_type(),
                )});
            }
            quote!(nu_protocol::Type::Record(
                std::vec![#(#fields_ts),*].into_boxed_slice()
            ))
        }
        (f @ Fields::Unnamed(fields), _) => {
            attributes::deny_fields(f)?;
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

    Ok(quote! {
        fn expected_type() -> nu_protocol::Type {
            #ty
        }
    })
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
) -> Result {
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
/// The input value is expected to be a `Value::String` containing the name of the variant.
/// That string is defined by the [`NameResolver::resolve_ident`] method with the `default` value
/// being [`Case::Snake`].
///
/// If no matching variant is found, `ShellError::CantConvert` is returned.
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
///         let span = v.span();
///         let ty = v.get_type();
///
///         let s = v.into_string()?;
///         match s.as_str() {
///             "sunny" => std::result::Ok(Self::Sunny),
///             "cloudy" => std::result::Ok(Self::Cloudy),
///             "rain" => std::result::Ok(Self::Raining),
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
fn enum_from_value(data: &DataEnum, attrs: &[Attribute]) -> Result {
    let container_attrs = ContainerAttributes::parse_attrs(attrs.iter())?;
    let mut name_resolver = NameResolver::new();
    let arms: Vec<TokenStream2> = data
        .variants
        .iter()
        .map(|variant| {
            let member_attrs = MemberAttributes::parse_attrs(&variant.attrs)?;
            let ident = &variant.ident;
            let ident_s =
                name_resolver.resolve_ident(ident, &container_attrs, &member_attrs, Case::Snake)?;
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
        .collect::<Result<_>>()?;

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
/// This function handles parsing a `Value` into the corresponding struct or enum variant (`self`).
/// It takes three parameters: `fields`, `self_ident`, and `rename_all`.
///
/// - The `fields` parameter specifies the expected structure of the `Value`:
///   - Named fields expect a `Value::Record`.
///   - Unnamed fields expect a `Value::List`.
///   - A unit struct expects `Value::Nothing`.
///
/// For named fields, each field in the record is matched to a struct field.
/// The name matching uses the identifiers resolved by
/// [`NameResolver`](NameResolver::resolve_ident) with `default` being `None`.
///
/// The `self_ident` parameter is used to specify the identifier for the returned value.
/// For most structs, `Self` is sufficient, but `Self::Variant` may be needed for enum variants.
///
/// The `container_attrs` parameters, provided through `#[nu_value]` on the container, defines
/// global rules for the `FromValue` implementation.
/// This is used for the [`NameResolver`] to resolve the correct ident in the `Value`.
///
/// This function is more complex than the equivalent for `IntoValue` due to additional error
/// handling:
/// - If a named field is missing in the `Value`, `ShellError::CantFindColumn` is returned.
/// - For unit structs, if the value is not `Value::Nothing`, `ShellError::CantConvert` is returned.
///
/// The implementation avoids local variables for fields to prevent accidental shadowing, ensuring
/// that fields with similar names do not cause unexpected behavior.
/// This approach is not typically recommended in handwritten Rust, but it is acceptable for code
/// generation.
fn parse_value_via_fields(
    fields: &Fields,
    self_ident: impl ToTokens,
    container_attrs: &ContainerAttributes,
) -> Result {
    match fields {
        Fields::Named(fields) => {
            let mut name_resolver = NameResolver::new();
            let mut fields_ts: Vec<TokenStream2> = Vec::with_capacity(fields.named.len());
            for field in fields.named.iter() {
                let member_attrs = MemberAttributes::parse_attrs(&field.attrs)?;
                let ident = field.ident.as_ref().expect("named has idents");
                let ident_s =
                    name_resolver.resolve_ident(ident, container_attrs, &member_attrs, None)?;
                let ty = &field.ty;
                fields_ts.push(match (type_is_option(ty), member_attrs.default) {
                    (true, _) => quote! {
                        #ident: record
                            .remove(#ident_s)
                            .map(|v| <#ty as nu_protocol::FromValue>::from_value(v))
                            .transpose()?
                            .flatten()
                    },
                    (false, false) => quote! {
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
                    (false, true) => quote! {
                        #ident: record
                            .remove(#ident_s)
                            .map(|v| <#ty as nu_protocol::FromValue>::from_value(v))
                            .transpose()?
                            .unwrap_or_default()
                    },
                });
            }
            Ok(quote! {
                let span = v.span();
                let mut record = v.into_record()?;
                std::result::Result::Ok(#self_ident {#(#fields_ts),*})
            })
        }
        f @ Fields::Unnamed(fields) => {
            attributes::deny_fields(f)?;
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
            Ok(quote! {
                let span = v.span();
                let list = v.into_list()?;
                let mut deque: std::collections::VecDeque<_> = std::convert::From::from(list);
                std::result::Result::Ok(#self_ident(#(#fields),*))
            })
        }
        Fields::Unit => Ok(quote! {
            match v {
                nu_protocol::Value::Nothing {..} => Ok(#self_ident),
                v => std::result::Result::Err(nu_protocol::ShellError::CantConvert {
                    to_type: std::string::ToString::to_string(&<Self as nu_protocol::FromValue>::expected_type()),
                    from_type: std::string::ToString::to_string(&v.get_type()),
                    span: v.span(),
                    help: std::option::Option::None
                })
            }
        }),
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
