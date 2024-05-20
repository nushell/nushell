use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Data, DataEnum, DataStruct, DeriveInput, Fields, Generics, Ident};

pub fn derive_into_value(input: TokenStream2) -> syn::Result<TokenStream2> {
    let input: DeriveInput = syn::parse2(input)?;
    match input.data {
        Data::Struct(data_struct) => {
            Ok(struct_into_value(input.ident, data_struct, input.generics))
        }
        Data::Enum(data_enum) => Ok(enum_into_value(data_enum)),
        Data::Union(_) => todo!("throw some error"),
    }
}

fn struct_into_value(ident: Ident, data: DataStruct, generics: Generics) -> TokenStream2 {
    let fields: Vec<TokenStream2> = match data.fields {
        Fields::Named(fields) => fields
            .named
            .into_iter()
            .map(|field| {
                let ident = field.ident.expect("named fields have an ident");
                let field = ident.to_string();
                quote!(#field => nu_protocol::IntoValue::into_value(self.#ident, span))
            })
            .collect(),
        _ => todo!(),
    };

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    quote! {
        impl #impl_generics nu_protocol::IntoValue for #ident #ty_generics #where_clause {
            fn into_value(self, span: nu_protocol::Span) -> nu_protocol::Value {
                nu_protocol::Value::record(nu_protocol::record! {
                    #(#fields),*
                }, span)
            }
        }
    }
}

fn enum_into_value(input: DataEnum) -> TokenStream2 {
    todo!()
}
