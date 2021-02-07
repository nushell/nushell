#[allow(unused_imports)]
use super::parse_signature;
#[allow(unused_imports)]
use nu_errors::ParseError;
#[allow(unused_imports)]
use nu_protocol::{NamedType, PositionalType, Signature, SyntaxShape};
#[allow(unused_imports)]
use nu_source::{Span, Spanned, SpannedItem};
#[allow(unused_imports)]
use nu_test_support::nu;

#[test]
fn simple_def_with_params() {
    let name = "my_func";
    let sign = "[param1?: int, param2: string]";
    let (sign, err) = parse_signature(name, &sign.to_string().spanned(Span::new(0, 27)));
    assert!(err.is_none());
    assert_eq!(
        sign.positional,
        vec![
            (
                PositionalType::Optional("param1".into(), SyntaxShape::Int),
                "".into()
            ),
            (
                PositionalType::Mandatory("param2".into(), SyntaxShape::String),
                "".into()
            ),
        ]
    );
}

#[test]
fn simple_def_with_optional_param_without_type() {
    let name = "my_func";
    let sign = "[param1 ?, param2?]";
    let (sign, err) = parse_signature(name, &sign.to_string().spanned(Span::new(0, 27)));
    assert!(err.is_none());
    assert_eq!(
        sign.positional,
        vec![
            (
                PositionalType::Optional("param1".into(), SyntaxShape::Any),
                "".into()
            ),
            (
                PositionalType::Optional("param2".into(), SyntaxShape::Any),
                "".into()
            ),
        ]
    );
}

#[test]
fn simple_def_with_params_with_comment() {
    let name = "my_func";
    let sign = "[
        param1:path # My first param
        param2:number # My second param
        ]";
    let (sign, err) = parse_signature(name, &sign.to_string().spanned(Span::new(0, 64)));
    assert!(err.is_none());
    assert_eq!(
        sign.positional,
        vec![
            (
                PositionalType::Mandatory("param1".into(), SyntaxShape::FilePath),
                "My first param".into()
            ),
            (
                PositionalType::Mandatory("param2".into(), SyntaxShape::Number),
                "My second param".into()
            ),
        ]
    );
}

#[test]
fn simple_def_with_params_without_type() {
    let name = "my_func";
    let sign = "[
        param1 # My first param
        param2:number # My second param
        ]";
    let (sign, err) = parse_signature(name, &sign.to_string().spanned(Span::new(0, 0)));
    assert!(err.is_none());
    assert_eq!(
        sign.positional,
        vec![
            (
                PositionalType::Mandatory("param1".into(), SyntaxShape::Any),
                "My first param".into()
            ),
            (
                PositionalType::Mandatory("param2".into(), SyntaxShape::Number),
                "My second param".into()
            ),
        ]
    );
}

#[test]
fn oddly_but_correct_written_params() {
    let name = "my_func";
    let sign = "[
        param1 :int         #      param1

        param2 : number # My second param


        param4, param5:path  ,  param6 # param6
        ]";
    let (sign, err) = parse_signature(name, &sign.to_string().spanned(Span::new(0, 0)));
    assert!(err.is_none());
    assert_eq!(
        sign.positional,
        vec![
            (
                PositionalType::Mandatory("param1".into(), SyntaxShape::Int),
                "param1".into()
            ),
            (
                PositionalType::Mandatory("param2".into(), SyntaxShape::Number),
                "My second param".into()
            ),
            (
                PositionalType::Mandatory("param4".into(), SyntaxShape::Any),
                "".into()
            ),
            (
                PositionalType::Mandatory("param5".into(), SyntaxShape::FilePath),
                "".into()
            ),
            (
                PositionalType::Mandatory("param6".into(), SyntaxShape::Any),
                "param6".into()
            ),
        ]
    );
}

#[test]
fn err_wrong_dash_count() {
    let actual = nu!(
        cwd: ".",
        "def f [ --flag(--f)] { echo hi }"
    );
    assert!(actual.err.contains("single '-'"));
}

#[test]
fn err_wrong_dash_count2() {
    let actual = nu!(
        cwd: ".",
        "def f [ --flag(f)] { echo hi }"
    );
    assert!(actual.err.contains("'-'"));
}

#[test]
fn err_wrong_type() {
    let actual = nu!(
        cwd: ".",
        "def f [ param1:strig ] { echo hi }"
    );
    assert!(actual.err.contains("type"));
}

//For what ever reason, this gets reported as not used
#[allow(dead_code)]
fn assert_signature_has_flag(sign: &Signature, name: &str, type_: NamedType, comment: &str) {
    assert_eq!(
        Some((type_, comment.to_string())),
        sign.named.get(name).cloned()
    );
}

#[test]
fn simple_def_with_only_flags() {
    let name = "my_func";
    let sign = "[
        --list (-l) : path  # First flag
        --verbose : number # Second flag
        --all(-a) # My switch
        ]";
    let (sign, err) = parse_signature(name, &sign.to_string().spanned_unknown());
    assert!(err.is_none());
    assert_signature_has_flag(
        &sign,
        "list",
        NamedType::Optional(Some('l'), SyntaxShape::FilePath),
        "First flag",
    );
    assert_signature_has_flag(
        &sign,
        "verbose",
        NamedType::Optional(None, SyntaxShape::Number),
        "Second flag",
    );
    assert_signature_has_flag(&sign, "all", NamedType::Switch(Some('a')), "My switch");
}

#[test]
fn simple_def_with_params_and_flags() {
    let name = "my_func";
    let sign = "[
        --list (-l) : path  # First flag
        param1, param2:table # Param2 Doc
        --verbose # Second flag
        param3 : number,
        --flag3 # Third flag
        param4 ?: table # Optional Param
        ]";
    let (sign, err) = parse_signature(name, &sign.to_string().spanned_unknown());
    assert!(err.is_none());
    assert_signature_has_flag(
        &sign,
        "list",
        NamedType::Optional(Some('l'), SyntaxShape::FilePath),
        "First flag",
    );
    assert_signature_has_flag(&sign, "verbose", NamedType::Switch(None), "Second flag");
    assert_signature_has_flag(&sign, "flag3", NamedType::Switch(None), "Third flag");
    assert_eq!(
        sign.positional,
        vec![
            (
                PositionalType::Mandatory("param1".into(), SyntaxShape::Any),
                "".into()
            ),
            (
                PositionalType::Mandatory("param2".into(), SyntaxShape::Table),
                "Param2 Doc".into()
            ),
            (
                PositionalType::Mandatory("param3".into(), SyntaxShape::Number),
                "".into()
            ),
            (
                PositionalType::Optional("param4".into(), SyntaxShape::Table),
                "Optional Param".into()
            ),
        ]
    );
}

#[test]
fn simple_def_with_parameters_and_flags_no_delimiter() {
    let name = "my_func";
    let sign = "[ param1:int param2
            --force (-f) param3 # Param3
            ]";
    let (sign, err) = parse_signature(name, &sign.to_string().spanned_unknown());
    assert!(err.is_none());
    assert_signature_has_flag(&sign, "force", NamedType::Switch(Some('f')), "");
    assert_eq!(
        sign.positional,
        // --list (-l) : path  # First flag
        // param1, param2:table # Param2 Doc
        // --verbose # Second flag
        // param3 : number,
        // --flag3 # Third flag
        vec![
            (
                PositionalType::Mandatory("param1".into(), SyntaxShape::Int),
                "".into()
            ),
            (
                PositionalType::Mandatory("param2".into(), SyntaxShape::Any),
                "".into()
            ),
            (
                PositionalType::Mandatory("param3".into(), SyntaxShape::Any),
                "Param3".into()
            ),
        ]
    );
}

#[test]
fn simple_example_signature() {
    let name = "my_func";
    let sign = "[
        d:int          # The required d parameter
        --x (-x):string # The all powerful x flag
        --y (-y):int    # The accompanying y flag
        ]";
    let (sign, err) = parse_signature(name, &sign.to_string().spanned_unknown());
    assert!(err.is_none());
    assert_signature_has_flag(
        &sign,
        "x",
        NamedType::Optional(Some('x'), SyntaxShape::String),
        "The all powerful x flag",
    );
    assert_signature_has_flag(
        &sign,
        "y",
        NamedType::Optional(Some('y'), SyntaxShape::Int),
        "The accompanying y flag",
    );
    assert_eq!(
        sign.positional,
        vec![(
            PositionalType::Mandatory("d".into(), SyntaxShape::Int),
            "The required d parameter".into()
        )]
    );
}

#[test]
fn flag_withouth_space_between_longname_shortname() {
    let name = "my_func";
    let sign = "[
        --xxx(-x):string # The all powerful x flag
        ]";
    let (sign, err) = parse_signature(name, &sign.to_string().spanned_unknown());
    assert!(err.is_none());
    assert_signature_has_flag(
        &sign,
        "xxx",
        NamedType::Optional(Some('x'), SyntaxShape::String),
        "The all powerful x flag",
    );
}

#[test]
fn simple_def_with_rest_arg() {
    let name = "my_func";
    let sign = "[ ...rest]";
    let (sign, err) = parse_signature(name, &sign.to_string().spanned_unknown());
    assert!(err.is_none());
    assert_eq!(
        sign.rest_positional,
        Some((SyntaxShape::Any, "".to_string()))
    );
}

#[test]
fn simple_def_with_rest_arg_with_type_and_comment() {
    let name = "my_func";
    let sign = "[ ...rest:path # My super cool rest arg]";
    let (sign, err) = parse_signature(name, &sign.to_string().spanned_unknown());
    assert!(err.is_none());
    assert_eq!(
        sign.rest_positional,
        Some((SyntaxShape::FilePath, "My super cool rest arg".to_string()))
    );
}

#[test]
fn simple_def_with_param_flag_and_rest() {
    let name = "my_func";
    let sign = "[
        d:string          # The required d parameter
        --xxx(-x)         # The all powerful x flag
        --yyy (-y):int    #    The accompanying y flag
        ...rest:table # Another rest
        ]";
    let (sign, err) = parse_signature(name, &sign.to_string().spanned_unknown());
    assert!(err.is_none());
    assert_signature_has_flag(
        &sign,
        "xxx",
        NamedType::Switch(Some('x')),
        "The all powerful x flag",
    );
    assert_signature_has_flag(
        &sign,
        "yyy",
        NamedType::Optional(Some('y'), SyntaxShape::Int),
        "The accompanying y flag",
    );
    assert_eq!(
        sign.positional,
        vec![(
            PositionalType::Mandatory("d".into(), SyntaxShape::String),
            "The required d parameter".into()
        )]
    );
    assert_eq!(
        sign.rest_positional,
        Some((SyntaxShape::Table, "Another rest".to_string()))
    );
}
