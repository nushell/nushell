use convert_case::{Case, Casing};
use proc_macro2::{Span, TokenStream as TokenStream2};
use proc_macro_error::{Diagnostic, Level};
use quote::{quote, ToTokens};
use syn::{spanned::Spanned, Data, DataEnum, DataStruct, DeriveInput, Fields, Generics, Ident};

enum DeriveError {
    Syn(syn::parse::Error),
    UnsupportedUnions,
    UnsupportedEnums { fields_span: Span },
}

impl From<DeriveError> for Diagnostic {
    fn from(value: DeriveError) -> Self {
        match value {
            DeriveError::Syn(e) => Diagnostic::spanned(e.span(), Level::Error, e.to_string()),
            DeriveError::UnsupportedUnions => Diagnostic::new(
                Level::Error,
                "`FromValue` cannot be derived from unions".to_string(),
            )
            .help("consider refactoring to a struct or enum".to_string())
            .note("if you really need a union, consider opening an issue on Github".to_string()),
            DeriveError::UnsupportedEnums { fields_span } => Diagnostic::spanned(
                fields_span,
                Level::Error,
                "`FromValue` can only be derived from plain enums".to_string(),
            )
            .help(
                "consider refactoring your data type to a struct with a plain enum as a field"
                    .to_string(),
            )
            .note("more complex enums could be implemented in the future".to_string()),
        }
    }
}

pub fn derive_from_value(input: TokenStream2) -> Result<TokenStream2, impl Into<Diagnostic>> {
    let input: DeriveInput = syn::parse2(input).map_err(DeriveError::Syn)?;
    match input.data {
        Data::Struct(data_struct) => Ok(derive_struct_from_value(
            input.ident,
            data_struct,
            input.generics,
        )),
        Data::Enum(data_enum) => Ok(derive_enum_from_value(
            input.ident,
            data_enum,
            input.generics,
        )?),
        Data::Union(_) => Err(DeriveError::UnsupportedUnions),
    }
}

fn derive_struct_from_value(ident: Ident, data: DataStruct, generics: Generics) -> TokenStream2 {
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let from_value_impl = struct_from_value(&data);
    let expected_type_impl = struct_expected_type(&data.fields);
    quote! {
        #[automatically_derived]
        impl #impl_generics nu_protocol::FromValue for #ident #ty_generics #where_clause {
            #from_value_impl
            #expected_type_impl
        }
    }
}

fn struct_from_value(data: &DataStruct) -> TokenStream2 {
    let body = fields_from_record(&data.fields, quote!(Self));
    quote! {
        fn from_value(
            v: nu_protocol::Value
        ) -> std::result::Result<Self, nu_protocol::ShellError> {
            #body
        }
    }
}

fn struct_expected_type(fields: &Fields) -> TokenStream2 {
    let ty = match fields {
        Fields::Named(fields) => {
            let fields = fields.named.iter().map(|field| {
                let ident = field.ident.as_ref().expect("named has idents");
                let ident_s = ident.to_string();
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
        Fields::Unnamed(fields) => {
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
        Fields::Unit => quote!(nu_protocol::Type::Nothing),
    };

    quote! {
        fn expected_type() -> nu_protocol::Type {
            #ty
        }
    }
}

fn derive_enum_from_value(
    ident: Ident,
    data: DataEnum,
    generics: Generics,
) -> Result<TokenStream2, DeriveError> {
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let from_value_impl = enum_from_value(&data)?;
    // As variants are hard to type with the current type system, we use the
    // default impl for `expected_type`.
    Ok(quote! {
        #[automatically_derived]
        impl #impl_generics nu_protocol::FromValue for #ident #ty_generics #where_clause {
            #from_value_impl
        }
    })
}

fn enum_from_value(data: &DataEnum) -> Result<TokenStream2, DeriveError> {
    let arms: Vec<TokenStream2> = data
        .variants
        .iter()
        .map(|variant| {
            let ident = &variant.ident;
            let ident_s = format!("{ident}").as_str().to_case(Case::Snake);
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

fn fields_from_record(fields: &Fields, self_ident: impl ToTokens) -> TokenStream2 {
    match fields {
        Fields::Named(fields) => {
            let fields = fields.named.iter().map(|field| {
                // TODO: handle missing fields for Options as None
                let ident = field.ident.as_ref().expect("named has idents");
                let ident_s = ident.to_string();
                let ty = &field.ty;
                quote! {
                    #ident: <#ty as nu_protocol::FromValue>::from_value(
                        record
                            .remove(#ident_s)
                            .ok_or_else(|| nu_protocol::ShellError::CantFindColumn {
                                col_name: std::string::ToString::to_string(#ident_s),
                                span: std::option::Option::None,
                                src_span: span
                            })?,
                    )?
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
