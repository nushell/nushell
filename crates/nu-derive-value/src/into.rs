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
    Ok(quote! {
        #[automatically_derived]
        impl #impl_generics nu_protocol::IntoValue for #ident #ty_generics #where_clause {
            fn into_value(self, span: nu_protocol::Span) -> nu_protocol::Value {
                #record
            }
        }
    })
}

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
            quote!(nu_protocol::Value::list(vec![#(#items),*], span))
        }
        Fields::Unit => quote!(nu_protocol::Value::nothing(span)),
    }
}
