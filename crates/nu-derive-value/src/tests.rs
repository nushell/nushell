// These tests only check that the derive macros throw the relevant errors.
// Functionality of the derived types is tested in nu_protocol::value::test_derive.

use crate::error::DeriveError;
use crate::from::derive_from_value;
use crate::into::derive_into_value;
use quote::quote;

#[test]
fn unsupported_unions() {
    let input = quote! {
        #[nu_value]
        union SomeUnion {
            f1: u32,
            f2: f32,
        }
    };

    let from_res = derive_from_value(input.clone());
    assert!(
        matches!(from_res, Err(DeriveError::UnsupportedUnions)),
        "expected `DeriveError::UnsupportedUnions`, got {:?}",
        from_res
    );

    let into_res = derive_into_value(input);
    assert!(
        matches!(into_res, Err(DeriveError::UnsupportedUnions)),
        "expected `DeriveError::UnsupportedUnions`, got {:?}",
        into_res
    );
}

#[test]
fn unsupported_enums() {
    let input = quote! {
        #[nu_value(rename_all = "SCREAMING_SNAKE_CASE")]
        enum ComplexEnum {
            Unit,
            Unnamed(u32, f32),
            Named {
                u: u32,
                f: f32,
            }
        }
    };

    let from_res = derive_from_value(input.clone());
    assert!(
        matches!(from_res, Err(DeriveError::UnsupportedEnums { .. })),
        "expected `DeriveError::UnsupportedEnums`, got {:?}",
        from_res
    );

    let into_res = derive_into_value(input);
    assert!(
        matches!(into_res, Err(DeriveError::UnsupportedEnums { .. })),
        "expected `DeriveError::UnsupportedEnums`, got {:?}",
        into_res
    );
}

#[test]
fn unexpected_attribute() {
    let input = quote! {
        #[nu_value(what)]
        enum SimpleEnum {
            A,
            B,
        }
    };

    let from_res = derive_from_value(input.clone());
    assert!(
        matches!(from_res, Err(DeriveError::UnexpectedAttribute { .. })),
        "expected `DeriveError::UnexpectedAttribute`, got {:?}",
        from_res
    );

    let into_res = derive_into_value(input);
    assert!(
        matches!(into_res, Err(DeriveError::UnexpectedAttribute { .. })),
        "expected `DeriveError::UnexpectedAttribute`, got {:?}",
        into_res
    );
}

#[test]
fn deny_attribute_on_structs() {
    let input = quote! {
        #[nu_value]
        struct SomeStruct;
    };

    let from_res = derive_from_value(input.clone());
    assert!(
        matches!(from_res, Err(DeriveError::InvalidAttributePosition { .. })),
        "expected `DeriveError::InvalidAttributePosition`, got {:?}",
        from_res
    );

    let into_res = derive_into_value(input);
    assert!(
        matches!(into_res, Err(DeriveError::InvalidAttributePosition { .. })),
        "expected `DeriveError::InvalidAttributePosition`, got {:?}",
        into_res
    );
}

#[test]
fn deny_attribute_on_fields() {
    let input = quote! {
        struct SomeStruct {
            #[nu_value]
            field: ()
        }
    };

    let from_res = derive_from_value(input.clone());
    assert!(
        matches!(from_res, Err(DeriveError::InvalidAttributePosition { .. })),
        "expected `DeriveError::InvalidAttributePosition`, got {:?}",
        from_res
    );

    let into_res = derive_into_value(input);
    assert!(
        matches!(into_res, Err(DeriveError::InvalidAttributePosition { .. })),
        "expected `DeriveError::InvalidAttributePosition`, got {:?}",
        into_res
    );
}

#[test]
fn invalid_attribute_value() {
    let input = quote! {
        #[nu_value(rename_all = "CrazY-CasE")]
        enum SimpleEnum {
            A,
            B
        }
    };

    let from_res = derive_from_value(input.clone());
    assert!(
        matches!(from_res, Err(DeriveError::InvalidAttributeValue { .. })),
        "expected `DeriveError::InvalidAttributeValue`, got {:?}",
        from_res
    );

    let into_res = derive_into_value(input);
    assert!(
        matches!(into_res, Err(DeriveError::InvalidAttributeValue { .. })),
        "expected `DeriveError::InvalidAttributeValue`, got {:?}",
        into_res
    );
}
