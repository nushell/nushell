use syn::{ItemFn, parse::Nothing};

mod test;

#[proc_macro_attribute]
pub fn test(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    syn::parse_macro_input!(attr as Nothing);
    let item_fn = syn::parse_macro_input!(item as ItemFn);
    test::test(item_fn).into()
}
