use nu_protocol::{Flag, PositionalArg, Signature, SyntaxShape};

#[test]
fn test_signature() {
    let signature = Signature::new("new_signature");
    let from_build = Signature::build("new_signature");

    // asserting partial eq implementation
    assert_eq!(signature, from_build);

    // constructing signature with description
    let signature = Signature::new("signature").desc("example usage");
    assert_eq!(signature.usage, "example usage".to_string())
}

#[test]
fn test_signature_chained() {
    let signature = Signature::new("new_signature")
        .desc("description")
        .required("required", SyntaxShape::String, "required description")
        .optional("optional", SyntaxShape::String, "optional description")
        .required_named(
            "req_named",
            SyntaxShape::String,
            "required named description",
            Some('r'),
        )
        .named("named", SyntaxShape::String, "named description", Some('n'))
        .switch("switch", "switch description", None)
        .rest("rest", SyntaxShape::String, "rest description");

    assert_eq!(signature.required_positional.len(), 1);
    assert_eq!(signature.optional_positional.len(), 1);
    assert_eq!(signature.named.len(), 4); // The 3 above + help
    assert!(signature.rest_positional.is_some());
    assert_eq!(signature.get_shorts(), vec!['h', 'r', 'n']);
    assert_eq!(
        signature.get_names(),
        vec!["help", "req_named", "named", "switch"]
    );
    assert_eq!(signature.num_positionals(), 2);

    assert_eq!(
        signature.get_positional(0),
        Some(PositionalArg {
            name: "required".to_string(),
            desc: "required description".to_string(),
            shape: SyntaxShape::String,
            var_id: None
        })
    );
    assert_eq!(
        signature.get_positional(1),
        Some(PositionalArg {
            name: "optional".to_string(),
            desc: "optional description".to_string(),
            shape: SyntaxShape::String,
            var_id: None
        })
    );
    assert_eq!(
        signature.get_positional(2),
        Some(PositionalArg {
            name: "rest".to_string(),
            desc: "rest description".to_string(),
            shape: SyntaxShape::String,
            var_id: None
        })
    );

    assert_eq!(
        signature.get_long_flag("req_named"),
        Some(Flag {
            long: "req_named".to_string(),
            short: Some('r'),
            arg: Some(SyntaxShape::String),
            required: true,
            desc: "required named description".to_string(),
            var_id: None
        })
    );

    assert_eq!(
        signature.get_short_flag('r'),
        Some(Flag {
            long: "req_named".to_string(),
            short: Some('r'),
            arg: Some(SyntaxShape::String),
            required: true,
            desc: "required named description".to_string(),
            var_id: None
        })
    );
}

#[test]
#[should_panic(expected = "There may be duplicate short flags, such as -h")]
fn test_signature_same_short() {
    // Creating signature with same short name should panic
    Signature::new("new_signature")
        .required_named(
            "required_named",
            SyntaxShape::String,
            "required named description",
            Some('n'),
        )
        .named("named", SyntaxShape::String, "named description", Some('n'));
}

#[test]
#[should_panic(expected = "There may be duplicate name flags, such as --help")]
fn test_signature_same_name() {
    // Creating signature with same short name should panic
    Signature::new("new_signature")
        .required_named(
            "name",
            SyntaxShape::String,
            "required named description",
            Some('r'),
        )
        .named("name", SyntaxShape::String, "named description", Some('n'));
}
