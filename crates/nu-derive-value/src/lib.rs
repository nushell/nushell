use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use proc_macro_error::proc_macro_error;

mod into;

#[proc_macro_derive(IntoValue)]
#[proc_macro_error]
pub fn derive_into_value(input: TokenStream) -> TokenStream {
    let input = TokenStream2::from(input);
    let output = into::derive_into_value(input).unwrap();
    TokenStream::from(output)
}

#[proc_macro_derive(FromValue)]
#[proc_macro_error]
pub fn derive_from_value(input: TokenStream) -> TokenStream {
    let _ = input;
    todo!()
}
