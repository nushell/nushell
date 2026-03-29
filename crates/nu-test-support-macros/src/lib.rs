use syn::{ItemFn, parse::Nothing};

mod test;
mod test_cell_path;

#[proc_macro_attribute]
pub fn test(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    syn::parse_macro_input!(attr as Nothing);
    let item_fn = syn::parse_macro_input!(item as ItemFn);
    test::test(item_fn).into()
}

#[proc_macro]
pub fn test_cell_path(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
    test_cell_path::test_cell_path(tokens.into())
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}
