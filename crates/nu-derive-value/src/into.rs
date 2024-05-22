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
                "`IntoValue` cannot be derived from unions".to_string(),
            )
            .help("consider refactoring to a struct or enum".to_string())
            .note("if you really need a union, consider opening an issue on Github".to_string()),
        }
    }
}

pub fn derive_into_value(input: TokenStream2) -> Result<TokenStream2, impl Into<Diagnostic>> {
    let input: DeriveInput = syn::parse2(input).map_err(DeriveError::Syn)?;
    match input.data {
        Data::Struct(data_struct) => {
            Ok(struct_into_value(input.ident, data_struct, input.generics))
        }
        Data::Enum(data_enum) => Ok(enum_into_value(input.ident, data_enum, input.generics)),
        Data::Union(_) => Err(DeriveError::Unsupported),
    }
}

fn struct_into_value(ident: Ident, data: DataStruct, generics: Generics) -> TokenStream2 {
    let record = match &data.fields {
        Fields::Named(fields) => {
            let accessor = fields
                .named
                .iter()
                .map(|field| field.ident.clone().expect("named has idents"))
                .map(|ident| quote!(self.#ident));
            fields_to_record(&data.fields, accessor)
        }
        Fields::Unnamed(fields) => {
            let accessor = fields
                .unnamed
                .iter()
                .enumerate()
                .map(|(n, _)| Index::from(n))
                .map(|index| quote!(self.#index));
            fields_to_record(&data.fields, accessor)
        }
        Fields::Unit => quote!(nu_protocol::Value::nothing(span)),
    };
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    quote! {
        impl #impl_generics nu_protocol::IntoValue for #ident #ty_generics #where_clause {
            fn into_value(self, span: nu_protocol::Span) -> nu_protocol::Value {
                #record
            }
        }
    }
}

fn enum_into_value(ident: Ident, data: DataEnum, generics: Generics) -> TokenStream2 {
    let arms: Vec<TokenStream2> = data.variants.into_iter().map(|variant| {
        let ident = variant.ident;
        let ident_s = format!("{ident}").as_str().to_case(Case::Snake);
        match &variant.fields {
            Fields::Named(fields) => {
                let accessor = fields.named.iter().map(|field| field.ident.clone().expect("named fields have an ident"));
                let fields: Vec<Ident> = accessor.clone().collect();
                let content = fields_to_record(&variant.fields, accessor);
                quote! {
                    Self::#ident {#(#fields),*} => nu_protocol::Value::record(nu_protocol::record! {
                        "$type" => nu_protocol::Value::string(#ident_s, span),
                        "$content" => #content
                    }, span)
                }
            }
            Fields::Unnamed(fields) => {
                let accessor = fields.unnamed.iter().enumerate().map(|(n, _)| format_ident!("v{n}"));
                let fields: Vec<Ident> = accessor.clone().collect();
                let content = fields_to_record(&variant.fields, accessor);
                quote! {
                    Self::#ident(#(#fields),*) => nu_protocol::Value::record(nu_protocol::record! {
                        "$type" => nu_protocol::Value::string(#ident_s, span),
                        "$content" => #content
                    }, span)
                }
            }
            Fields::Unit => quote! {
                Self::#ident => nu_protocol::Value::record(nu_protocol::record! {
                    "$type" => nu_protocol::Value::string(#ident_s, span)
                }, span)
            }
        }
    }).collect();

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    quote! {
        impl #impl_generics nu_protocol::IntoValue for #ident #ty_generics #where_clause {
            fn into_value(self, span: nu_protocol::Span) -> nu_protocol::Value {
                match self {
                    #(#arms),*
                }
            }
        }
    }
}

fn fields_to_record(
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
                    let ident = field.ident.clone().expect("named has idents");
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
            quote!(nu_protocol::Value::list(vec![#(#items),*], span))
        }
        Fields::Unit => quote!(nu_protocol::Value::nothing(span)),
    }
}
