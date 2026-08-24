use quote::quote;
use std::mem;
use syn::{
    Attribute, Expr, FnArg, Ident, ItemFn, Lit, LitBool, LitStr, Meta, MetaNameValue, Pat, Path,
    Token, parse::ParseStream, spanned::Spanned,
};

pub fn test(mut item_fn: ItemFn) -> proc_macro2::TokenStream {
    let attrs = match TestAttributes::try_from(mem::take(&mut item_fn.attrs)) {
        Ok(attrs) => attrs,
        Err(err) => return err.to_compile_error(),
    };
    let attr_rest = attrs.rest;
    let dependencies = attrs.dependencies;

    let args = match TestArgs::try_from_iter(mem::take(&mut item_fn.sig.inputs).into_iter()) {
        Ok(args) => args,
        Err(err) => return err.to_compile_error(),
    };

    let arg_array = [args.playground.as_ref()];

    // reorder arguments of test function to inject arguments in correct order
    item_fn.sig.inputs = arg_array
        .into_iter()
        .flatten()
        .map(|(arg, _)| arg)
        .cloned()
        .collect();

    let playground = args.playground.as_ref().map(|(_, ident)| quote! {
        let #ident = match ::nu_test_support::playground::Playground::new(
            MODULE_PATH_WITHOUT_CRATE,
            ::std::env!("CARGO_PKG_NAME"),
            ::std::env!("CARGO_CRATE_NAME"),
        ) {
            ::std::result::Result::Err(err) => return ::nu_test_support::harness::IntoTestResult::into_test_result(Err(err)),
            ::std::result::Result::Ok(ok) => ok,
        };
    }).into_iter();

    let fn_ident = &item_fn.sig.ident;
    let fn_args = arg_array.into_iter().flatten().map(|(_, ident)| ident);

    let run_in_serial = match attrs.run_in_serial {
        Some(true) => true,
        Some(false) => false,
        None => false,
    };

    let ignore_status = match attrs.ignore {
        (false, _) => quote!(IgnoreStatus::Run),
        (true, None) => quote!(IgnoreStatus::Ignore),
        (true, Some(msg)) => quote!(IgnoreStatus::IgnoreWithReason(Cow::Borrowed(#msg))),
    };

    let panic_expectation = match attrs.should_panic {
        (false, _) => quote!(PanicExpectation::ShouldNotPanic),
        (true, None) => quote!(PanicExpectation::ShouldPanic),
        (true, Some(msg)) => quote!(PanicExpectation::ShouldPanicWithExpected(Cow::Borrowed(#msg))),
    };

    let experimental_options = attrs.experimental_options.into_iter().map(|(path, lit)| {
        let lit = lit.map(|lit| lit.value).unwrap_or(true);
        quote!((&#path, #lit))
    });

    let environment_variables = attrs.environment_variables.into_iter().map(|(key, value)| {
        let key = key.to_string();
        quote!((#key, #value))
    });

    quote! {
        #[::core::prelude::v1::test]
        fn #fn_ident() {}

        mod #fn_ident {
            use super::*;
            use ::nu_test_support::harness::{
                Cow,
                IgnoreStatus,
                Extra,
                PanicExpectation,
                Test,
                TestFnHandle,
                TestMeta,
                TestResult,
            };

            const MODULE_PATH_WITHOUT_CRATE: &str = ::nu_test_support::module_path_without_crate!();

            fn wrapper() -> TestResult {
                #(#playground)*
                ::nu_test_support::harness::IntoTestResult::into_test_result(#fn_ident(#(#fn_args),*))
            }

            #[::nu_test_support::collect_test(::nu_test_support::harness::TESTS)]
            #[linkme(crate = ::nu_test_support::harness::linkme)]
            static TEST: Test<Extra> =
                Test::new(
                    TestFnHandle::from_const_fn(wrapper),
                    TestMeta {
                        name: Cow::Borrowed(MODULE_PATH_WITHOUT_CRATE),
                        ignore: #ignore_status,
                        should_panic: #panic_expectation,
                        origin: ::nu_test_support::harness::origin!(),
                        extra: Extra {
                            run_in_serial: #run_in_serial,
                            experimental_options: &[#(#experimental_options),*],
                            environment_variables: &[#(#environment_variables),*],
                            dependencies: &[#(#dependencies),*],
                        }
                    }
                );
        }

        #(#attr_rest)*
        #item_fn
    }
}

#[derive(Default)]
pub struct TestAttributes {
    pub ignore: (bool, Option<LitStr>),
    pub should_panic: (bool, Option<LitStr>),
    pub run_in_serial: Option<bool>,
    pub experimental_options: Vec<(Path, Option<LitBool>)>,
    pub environment_variables: Vec<(Ident, Expr)>,
    pub dependencies: Vec<Expr>,
    pub rest: Vec<Attribute>,
}

impl TryFrom<Vec<Attribute>> for TestAttributes {
    type Error = syn::Error;

    fn try_from(attrs: Vec<Attribute>) -> Result<Self, Self::Error> {
        let mut test_attrs = TestAttributes::default();

        for attr in attrs {
            let Some(ident) = attr.path().get_ident() else {
                test_attrs.rest.push(attr);
                continue;
            };

            match ident.to_string().as_str() {
                "ignore" => match attr.meta {
                    Meta::Path(_) => test_attrs.ignore.0 = true,

                    Meta::NameValue(MetaNameValue { value, .. }) => match value {
                        Expr::Lit(expr_lit) => match expr_lit.lit {
                            Lit::Str(lit_str) => {
                                test_attrs.ignore.0 = true;
                                test_attrs.ignore.1 = Some(lit_str);
                            }
                            other => {
                                return Err(syn::Error::new(
                                    other.span(),
                                    "invalid #[ignore = ...] value, expected a string like #[ignore = \"reason\"]",
                                ));
                            }
                        },
                        other => {
                            return Err(syn::Error::new(
                                other.span(),
                                "invalid #[ignore = ...] value, expected a string literal like #[ignore = \"reason\"]",
                            ));
                        }
                    },

                    Meta::List(meta_list) => {
                        return Err(syn::Error::new(
                            meta_list.span(),
                            "invalid #[ignore(...)] form. Use #[ignore] or #[ignore = \"reason\"]",
                        ));
                    }
                },

                "should_panic" => match attr.meta {
                    Meta::Path(_) => test_attrs.should_panic.0 = true,

                    Meta::List(meta_list) => meta_list.parse_nested_meta(|meta| {
                        if meta.path.is_ident("expected") {
                            let value = meta.value()?;
                            let expected: LitStr = value.parse()?;
                            test_attrs.should_panic.0 = true;
                            test_attrs.should_panic.1 = Some(expected);
                            Ok(())
                        } else {
                            Err(syn::Error::new(
                                meta.path.span(),
                                "unknown argument for #[should_panic(...)]. Only `expected = \"...\"` is supported",
                            ))
                        }
                    })?,

                    Meta::NameValue(nv) => {
                        return Err(syn::Error::new(
                            nv.span(),
                            "invalid #[should_panic = ...] form. Use #[should_panic] or #[should_panic(expected = \"...\")]",
                        ));
                    }
                },

                "serial" => match attr.meta {
                    Meta::Path(_) => test_attrs.run_in_serial = Some(true),

                    Meta::NameValue(nv) => match nv.value {
                        Expr::Lit(expr_lit) => match expr_lit.lit {
                            Lit::Bool(b) => test_attrs.run_in_serial = Some(b.value),
                            other => {
                                return Err(syn::Error::new(
                                    other.span(),
                                    "invalid #[serial = ...] value, expected a boolean like #[serial = true] or #[serial = false]",
                                ));
                            }
                        },
                        other => {
                            return Err(syn::Error::new(
                                other.span(),
                                "invalid #[serial = ...] value, expected a boolean literal",
                            ));
                        }
                    },

                    Meta::List(meta_list) => {
                        return Err(syn::Error::new(
                            meta_list.span(),
                            "invalid #[serial(...)] form. Use #[serial] or #[serial = true|false]",
                        ));
                    }
                },

                "exp" | "experimental_options" => {
                    fn parse(input: ParseStream) -> syn::Result<Vec<(Path, Option<LitBool>)>> {
                        Ok(input
                            .parse_terminated(
                                |input| {
                                    let path: Path = input.parse()?;
                                    if !input.peek(Token![=]) {
                                        return Ok((path, None));
                                    }
                                    let _: Token![=] = input.parse()?;
                                    let value: LitBool = input.parse()?;
                                    Ok((path, Some(value)))
                                },
                                Token![,],
                            )?
                            .into_iter()
                            .collect())
                    }

                    let options = attr.parse_args_with(parse)?;
                    test_attrs.experimental_options.extend(options);
                }

                "env" | "environment_variables" => {
                    fn parse(input: ParseStream) -> syn::Result<Vec<(Ident, Expr)>> {
                        Ok(input
                            .parse_terminated(
                                |input| {
                                    let key: Ident = input.parse()?;
                                    let _: Token![=] = input.parse()?;
                                    let value: Expr = input.parse()?;
                                    Ok((key, value))
                                },
                                Token![,],
                            )?
                            .into_iter()
                            .collect())
                    }

                    let envs = attr.parse_args_with(parse)?;
                    test_attrs.environment_variables.extend(envs);
                }

                "deps" | "dependencies" => {
                    fn parse(input: ParseStream) -> syn::Result<Vec<Expr>> {
                        Ok(input
                            .parse_terminated(|input| input.parse(), Token![,])?
                            .into_iter()
                            .collect())
                    }

                    let dependencies = attr.parse_args_with(parse)?;
                    test_attrs.dependencies.extend(dependencies);
                }

                _ => test_attrs.rest.push(attr),
            }
        }

        Ok(test_attrs)
    }
}

#[derive(Default)]
pub struct TestArgs {
    pub playground: Option<(FnArg, Ident)>,
}

impl TestArgs {
    pub fn try_from_iter(iter: impl Iterator<Item = FnArg>) -> syn::Result<Self> {
        let mut args = TestArgs::default();

        for arg in iter {
            let pat_type = match &arg {
                FnArg::Receiver(_) => {
                    return Err(syn::Error::new_spanned(arg, "unexpected self parameter"));
                }
                FnArg::Typed(pat_type) => pat_type,
            };

            let Pat::Ident(pat_ident) = &*pat_type.pat else {
                return Err(syn::Error::new(
                    pat_type.pat.span(),
                    "expected single ident",
                ));
            };

            let ident = pat_ident.ident.clone();
            match ident.to_string().as_str() {
                "playground" | "play" | "pg" | "_playground" => {
                    args.playground = Some((arg, ident.clone()))
                }
                _ => {
                    return Err(syn::Error::new_spanned(
                        ident,
                        "unknown arg name for injection, expected `playground`",
                    ));
                }
            }
        }

        Ok(args)
    }
}
