use nu_protocol::{Flag, PositionalArg, Signature, SyntaxShape, Type, TypeSet};

#[test]
fn test_signature() {
    let signature = Signature::new("new_signature");
    let from_build = Signature::build("new_signature");

    // asserting partial eq implementation
    assert_eq!(signature, from_build);

    // constructing signature with description
    let signature = Signature::new("signature").description("example description");
    assert_eq!(signature.description, "example description".to_string())
}

#[test]
fn test_signature_chained() {
    let signature = Signature::new("new_signature")
        .description("description")
        .required("required", SyntaxShape::String, "Required description.")
        .optional("optional", SyntaxShape::String, "Optional description.")
        .required_named(
            "req-named",
            SyntaxShape::String,
            "Required named description.",
            Some('r'),
        )
        .named(
            "named",
            SyntaxShape::String,
            "Named description.",
            Some('n'),
        )
        .switch("switch", "Switch description.", None)
        .rest("rest", SyntaxShape::String, "Rest description.");

    assert_eq!(signature.required_positional.len(), 1);
    assert_eq!(signature.optional_positional.len(), 1);
    assert_eq!(signature.named.len(), 3);
    assert!(signature.rest_positional.is_some());
    assert_eq!(signature.get_shorts(), vec!['r', 'n']);
    assert_eq!(signature.get_names(), vec!["req-named", "named", "switch"]);
    assert_eq!(signature.num_positionals(), 2);

    assert_eq!(
        signature.get_positional(0),
        Some(&PositionalArg {
            name: "required".to_string(),
            desc: "Required description.".to_string(),
            shape: SyntaxShape::String,
            var_id: None,
            default_value: None,
            completion: None,
        })
    );
    assert_eq!(
        signature.get_positional(1),
        Some(&PositionalArg {
            name: "optional".to_string(),
            desc: "Optional description.".to_string(),
            shape: SyntaxShape::String,
            var_id: None,
            default_value: None,
            completion: None,
        })
    );
    assert_eq!(
        signature.get_positional(2),
        Some(&PositionalArg {
            name: "rest".to_string(),
            desc: "Rest description.".to_string(),
            shape: SyntaxShape::String,
            var_id: None,
            default_value: None,
            completion: None,
        })
    );

    assert_eq!(
        signature.get_long_flag("req-named"),
        Some(Flag {
            long: "req-named".to_string(),
            short: Some('r'),
            arg: Some(SyntaxShape::String),
            required: true,
            desc: "Required named description.".to_string(),
            var_id: None,
            default_value: None,
            completion: None,
        })
    );

    assert_eq!(
        signature.get_short_flag('r'),
        Some(Flag {
            long: "req-named".to_string(),
            short: Some('r'),
            arg: Some(SyntaxShape::String),
            required: true,
            desc: "Required named description.".to_string(),
            var_id: None,
            default_value: None,
            completion: None,
        })
    );
}

#[test]
#[should_panic(expected = "There may be duplicate short flags for '-n'")]
fn test_signature_same_short() {
    // Creating signature with same short name should panic
    Signature::new("new_signature")
        .required_named(
            "required-named",
            SyntaxShape::String,
            "Required named description.",
            Some('n'),
        )
        .named(
            "named",
            SyntaxShape::String,
            "Named description.",
            Some('n'),
        );
}

#[test]
#[should_panic(expected = "There may be duplicate name flags for '--name'")]
fn test_signature_same_name() {
    // Creating signature with same short name should panic
    Signature::new("new-signature")
        .required_named(
            "name",
            SyntaxShape::String,
            "Required named description.",
            Some('r'),
        )
        .named("name", SyntaxShape::String, "Named description.", Some('n'));
}

#[test]
fn test_signature_round_trip() {
    let signature = Signature::new("new_signature")
        .description("description")
        .required("first", SyntaxShape::String, "First required.")
        .required("second", SyntaxShape::Int, "Second required.")
        .optional("optional", SyntaxShape::String, "Optional description.")
        .required_named(
            "req-named",
            SyntaxShape::String,
            "Required named description.",
            Some('r'),
        )
        .named(
            "named",
            SyntaxShape::String,
            "Named description.",
            Some('n'),
        )
        .switch("switch", "Switch description.", None)
        .rest("rest", SyntaxShape::String, "Rest description.")
        .category(nu_protocol::Category::Conversions);

    let string = serde_json::to_string_pretty(&signature).unwrap();
    let returned: Signature = serde_json::from_str(&string).unwrap();

    assert_eq!(signature.name, returned.name);
    assert_eq!(signature.description, returned.description);
    assert_eq!(signature.extra_description, returned.extra_description);
    assert_eq!(signature.is_filter, returned.is_filter);
    assert_eq!(signature.category, returned.category);

    signature
        .required_positional
        .iter()
        .zip(returned.required_positional.iter())
        .for_each(|(lhs, rhs)| assert_eq!(lhs, rhs));

    signature
        .optional_positional
        .iter()
        .zip(returned.optional_positional.iter())
        .for_each(|(lhs, rhs)| assert_eq!(lhs, rhs));

    signature
        .named
        .iter()
        .zip(returned.named.iter())
        .for_each(|(lhs, rhs)| assert_eq!(lhs, rhs));

    assert_eq!(signature.rest_positional, returned.rest_positional,);
}

fn into_string_like_signature() -> Signature {
    Signature::new("into string").input_output_types(vec![
        (Type::Int, Type::String),
        (Type::custom("semver"), Type::String),
        (Type::Any, Type::String),
        (
            Type::List(Box::new(Type::Any)),
            Type::List(Box::new(Type::String)),
        ),
        (Type::table(), Type::table()),
        (Type::record(), Type::record()),
    ])
}

#[test]
fn get_output_type_prefers_equal_over_any_and_structured() {
    let sig = into_string_like_signature();

    assert_eq!(
        sig.get_output_type(Some(&Type::custom("semver"))),
        Some(Type::String)
    );
    assert_eq!(sig.get_output_type(Some(&Type::Int)), Some(Type::String));
    assert_eq!(
        sig.get_output_type(Some(&Type::record())),
        Some(Type::record())
    );
}

#[test]
fn get_output_type_prefers_list_pair_over_structured_when_any_is_absent() {
    // `any` compared with `any` is a subtype, not equal, so a `list<any>` pair
    // ties with an `any` pair. Without `any`, list input keeps `list<string>`.
    let sig = Signature::new("into string").input_output_types(vec![
        (Type::custom("semver"), Type::String),
        (
            Type::List(Box::new(Type::Any)),
            Type::List(Box::new(Type::String)),
        ),
        (Type::table(), Type::table()),
        (Type::record(), Type::record()),
    ]);

    assert_eq!(
        sig.get_output_type(Some(&Type::list(Type::Any))),
        Some(Type::list(Type::String))
    );
    assert_eq!(
        sig.get_output_type(Some(&Type::list(Type::custom("semver")))),
        Some(Type::list(Type::String))
    );
}

#[test]
fn get_output_type_prefers_lattice_when_input_is_unioned_with_nothing() {
    let sig = into_string_like_signature();
    let input = Type::custom("semver").union(Type::Nothing);

    assert_eq!(sig.get_output_type(Some(&input)), Some(Type::String));
}

#[test]
fn get_output_type_returns_none_when_no_pair_matches() {
    let sig = Signature::new("only-int").input_output_types(vec![(Type::Int, Type::String)]);

    assert_eq!(sig.get_output_type(Some(&Type::Bool)), None);
}
