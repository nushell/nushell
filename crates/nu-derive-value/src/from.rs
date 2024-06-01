use convert_case::{Case, Casing};
use proc_macro2::TokenStream as TokenStream2;
use proc_macro_error::{Diagnostic, Level};
use quote::{quote, ToTokens};
use syn::{Data, DataEnum, DataStruct, DeriveInput, Fields, Generics, Ident};

enum DeriveError {
    Syn(syn::parse::Error),
    Unsupported,
}

impl From<DeriveError> for Diagnostic {
    fn from(value: DeriveError) -> Self {
        match value {
            DeriveError::Syn(e) => Diagnostic::spanned(e.span(), Level::Error, e.to_string()),
            DeriveError::Unsupported => Diagnostic::new(
                Level::Error,
                "`FromValue` cannot be derived from unions".to_string(),
            )
            .help("consider refactoring to a struct or enum".to_string())
            .note("if you really need a union, consider opening an issue on Github".to_string()),
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
        )),
        Data::Union(_) => Err(DeriveError::Unsupported),
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
            v: nu_protocol::Value,
            call_span: nu_protocol::Span
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
            iter.next().map(|_| template.push_str("{}"));
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

fn derive_enum_from_value(ident: Ident, data: DataEnum, generics: Generics) -> TokenStream2 {
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let from_value_impl = enum_from_value(&data);
    // As variants are hard to type with the current type system, we use the
    // default impl for `expected_type`.
    quote! {
        #[automatically_derived]
        impl #impl_generics nu_protocol::FromValue for #ident #ty_generics #where_clause {
            #from_value_impl
        }
    }
}

fn enum_from_value(data: &DataEnum) -> TokenStream2 {
    let arms = data.variants.iter().map(|variant| {
        let ident = &variant.ident;
        let ident_s = format!("{ident}").as_str().to_case(Case::Snake);
        let fields = fields_from_record(&variant.fields, quote!(Self::#ident));
        quote!(#ident_s => {#fields})
    });

    quote! {
        fn from_value(
            v: nu_protocol::Value,
            call_span: nu_protocol::Span
        ) -> std::result::Result<Self, nu_protocol::ShellError> {
            let span = v.span();
            let mut record = v.into_record()?;

            let ty = record.remove("type").ok_or_else(|| nu_protocol::ShellError::CantFindColumn {
                col_name: std::string::ToString::to_string("type"),
                span: call_span,
                src_span: span
            })?;
            let ty = ty.into_string()?;

            // This allows unit variants to resolve without the "content" field
            // in the record.
            let v = record
                .remove("content")
                .unwrap_or_else(|| nu_protocol::Value::nothing(span));

            match ty.as_str() {
                #(#arms),*
                _ => std::result::Result::Err(nu_protocol::ShellError::CantConvert {
                    to_type: std::string::ToString::to_string(
                        &<Self as nu_protocol::FromValue>::expected_type()
                    ),
                    from_type: ty,
                    span: span,
                    help: std::option::Option::None
                }),
            }
        }
    }
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
                                span: call_span,
                                src_span: span
                            })?,
                        call_span,
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
                                span: call_span,
                                src_span: span
                            })?,
                        call_span,
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
