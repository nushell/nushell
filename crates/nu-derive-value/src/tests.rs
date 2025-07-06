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
        "expected `DeriveError::UnsupportedUnions`, got {from_res:?}"
    );

    let into_res = derive_into_value(input);
    assert!(
        matches!(into_res, Err(DeriveError::UnsupportedUnions)),
        "expected `DeriveError::UnsupportedUnions`, got {into_res:?}"
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
        "expected `DeriveError::UnsupportedEnums`, got {from_res:?}"
    );

    let into_res = derive_into_value(input);
    assert!(
        matches!(into_res, Err(DeriveError::UnsupportedEnums { .. })),
        "expected `DeriveError::UnsupportedEnums`, got {into_res:?}"
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
        "expected `DeriveError::UnexpectedAttribute`, got {from_res:?}"
    );

    let into_res = derive_into_value(input);
    assert!(
        matches!(into_res, Err(DeriveError::UnexpectedAttribute { .. })),
        "expected `DeriveError::UnexpectedAttribute`, got {into_res:?}"
    );
}

#[test]
fn unexpected_attribute_on_struct_field() {
    let input = quote! {
        struct SimpleStruct {
            #[nu_value(what)]
            field_a: i32,
            field_b: String,
        }
    };

    let from_res = derive_from_value(input.clone());
    assert!(
        matches!(from_res, Err(DeriveError::UnexpectedAttribute { .. })),
        "expected `DeriveError::UnexpectedAttribute`, got {from_res:?}"
    );

    let into_res = derive_into_value(input);
    assert!(
        matches!(into_res, Err(DeriveError::UnexpectedAttribute { .. })),
        "expected `DeriveError::UnexpectedAttribute`, got {into_res:?}"
    );
}

#[test]
fn unexpected_attribute_on_enum_variant() {
    let input = quote! {
        enum SimpleEnum {
            #[nu_value(what)]
            A,
            B,
        }
    };

    let from_res = derive_from_value(input.clone());
    assert!(
        matches!(from_res, Err(DeriveError::UnexpectedAttribute { .. })),
        "expected `DeriveError::UnexpectedAttribute`, got {from_res:?}"
    );

    let into_res = derive_into_value(input);
    assert!(
        matches!(into_res, Err(DeriveError::UnexpectedAttribute { .. })),
        "expected `DeriveError::UnexpectedAttribute`, got {into_res:?}"
    );
}

#[test]
fn invalid_attribute_position_in_tuple_struct() {
    let input = quote! {
        struct SimpleTupleStruct(
            #[nu_value(what)]
            i32,
            String,
        );
    };

    let from_res = derive_from_value(input.clone());
    assert!(
        matches!(
            from_res,
            Err(DeriveError::InvalidAttributePosition { attribute_span: _ })
        ),
        "expected `DeriveError::InvalidAttributePosition`, got {from_res:?}"
    );

    let into_res = derive_into_value(input);
    assert!(
        matches!(
            into_res,
            Err(DeriveError::InvalidAttributePosition { attribute_span: _ })
        ),
        "expected `DeriveError::InvalidAttributePosition`, got {into_res:?}"
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
        "expected `DeriveError::InvalidAttributeValue`, got {from_res:?}"
    );

    let into_res = derive_into_value(input);
    assert!(
        matches!(into_res, Err(DeriveError::InvalidAttributeValue { .. })),
        "expected `DeriveError::InvalidAttributeValue`, got {into_res:?}"
    );
}

#[test]
fn non_unique_struct_keys() {
    let input = quote! {
        struct DuplicateStruct {
            #[nu_value(rename = "field")]
            some_field: (),
            field: (),
        }
    };

    let from_res = derive_from_value(input.clone());
    assert!(
        matches!(from_res, Err(DeriveError::NonUniqueName { .. })),
        "expected `DeriveError::NonUniqueName`, got {from_res:?}"
    );

    let into_res = derive_into_value(input);
    assert!(
        matches!(into_res, Err(DeriveError::NonUniqueName { .. })),
        "expected `DeriveError::NonUniqueName`, got {into_res:?}"
    );
}

#[test]
fn non_unique_enum_variants() {
    let input = quote! {
        enum DuplicateEnum {
            #[nu_value(rename = "variant")]
            SomeVariant,
            Variant
        }
    };

    let from_res = derive_from_value(input.clone());
    assert!(
        matches!(from_res, Err(DeriveError::NonUniqueName { .. })),
        "expected `DeriveError::NonUniqueName`, got {from_res:?}"
    );

    let into_res = derive_into_value(input);
    assert!(
        matches!(into_res, Err(DeriveError::NonUniqueName { .. })),
        "expected `DeriveError::NonUniqueName`, got {into_res:?}"
    );
}
