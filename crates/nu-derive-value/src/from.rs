use convert_case::{Case, Casing};
use proc_macro2::TokenStream as TokenStream2;
use proc_macro_error::{Diagnostic, Level};
use quote::{format_ident, quote, ToTokens};
use syn::{Data, DataEnum, DataStruct, DeriveInput, Fields, Generics, Ident, Index};

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
        Data::Enum(data_enum) => Ok(enum_from_value(input.ident, data_enum, input.generics)),
        Data::Union(_) => Err(DeriveError::Unsupported),
    }
}

fn derive_struct_from_value(ident: Ident, data: DataStruct, generics: Generics) -> TokenStream2 {
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let from_value_impl = struct_from_value(&data, &generics);
    let expected_type_impl = struct_expected_type(&data.fields);
    quote! {
        #[automatically_derived]
        impl #impl_generics nu_protocol::FromValue for #ident #ty_generics #where_clause {
            #from_value_impl
            #expected_type_impl
        }
    }
}

fn struct_from_value(data: &DataStruct, generics: &Generics) -> TokenStream2 {
    let this = match &data.fields {
        Fields::Named(fields) => {
            let fields = fields.named.iter().map(|field| {
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
            quote!(Self {#(#fields),*})
        }
        Fields::Unnamed(fields) => todo!(),
        Fields::Unit => todo!(),
    };

    quote! {
        fn from_value(v: nu_protocol::Value, call_span: nu_protocol::Span) -> std::result::Result<Self, nu_protocol::ShellError> {
            let span = v.span();
            let mut record = v.into_record()?;
            Ok(#this)
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

fn enum_from_value(ident: Ident, data: DataEnum, generics: Generics) -> TokenStream2 {
    todo!()
}
