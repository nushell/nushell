use nu_parser::*;
use nu_protocol::{
    DeclId, FilesizeUnit, ParseError, Signature, Span, SyntaxShape, Type, Unit,
    ast::{Argument, Expr, Expression, ExternalArgument, PathMember, Range},
    engine::{Command, EngineState, Stack, StateWorkingSet},
};
use rstest::rstest;

use mock::{Alias, AttrEcho, Const, Def, IfMocked, Let, Mut, ToCustom};

fn test_int(
    test_tag: &str,     // name of sub-test
    test: &[u8],        // input expression
    expected_val: Expr, // (usually Expr::{Int,String, Float}, not ::BinOp...
    expected_err: Option<&str>,
) // substring in error text
{
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let block = parse(&mut working_set, None, test, true);

    let err = working_set.parse_errors.first();

    if let Some(err_pat) = expected_err {
        if let Some(parse_err) = err {
            let act_err = format!("{parse_err:?}");
            assert!(
                act_err.contains(err_pat),
                "{test_tag}: expected err to contain {err_pat}, but actual error was {act_err}"
            );
        } else {
            assert!(
                err.is_some(),
                "{test_tag}: expected err containing {err_pat}, but no error returned"
            );
        }
    } else {
        assert!(err.is_none(), "{test_tag}: unexpected error {err:#?}");
        assert_eq!(block.len(), 1, "{test_tag}: result block length > 1");
        let pipeline = &block.pipelines[0];
        assert_eq!(
            pipeline.len(),
            1,
            "{test_tag}: got multiple result expressions, expected 1"
        );
        let element = &pipeline.elements[0];
        assert!(element.redirection.is_none());
        compare_rhs_binary_op(test_tag, &expected_val, &element.expr.expr);
    }
}

fn compare_rhs_binary_op(
    test_tag: &str,
    expected: &Expr, // the rhs expr we hope to see (::Int, ::Float, not ::B)
    observed: &Expr, // the Expr actually provided: can be ::Int, ::Float, ::String,
                     // or ::BinOp (in which case rhs is checked), or ::Call (in which case cmd is checked)
) {
    match observed {
        Expr::Int(..) | Expr::Float(..) | Expr::String(..) => {
            assert_eq!(
                expected, observed,
                "{test_tag}: Expected: {expected:#?}, observed {observed:#?}"
            );
        }
        Expr::BinaryOp(_, _, e) => {
            let observed_expr = &e.expr;
            // can't pattern match Box<Foo>, but can match the box, then deref in separate statement.
            assert_eq!(
                expected, observed_expr,
                "{test_tag}: Expected: {expected:#?}, observed: {observed:#?}"
            )
        }
        Expr::ExternalCall(e, _) => {
            let observed_expr = &e.expr;
            assert_eq!(
                expected, observed_expr,
                "{test_tag}: Expected: {expected:#?}, observed: {observed_expr:#?}"
            )
        }
        _ => {
            panic!("{test_tag}: Unexpected Expr:: variant returned, observed {observed:#?}");
        }
    }
}

#[test]
pub fn multi_test_parse_int() {
    struct Test<'a>(&'a str, &'a [u8], Expr, Option<&'a str>);

    // use test expression of form '0 + x' to force parse() to parse x as numeric.
    // if expression were just 'x', parse() would try other items that would mask the error we're looking for.
    let tests = vec![
        Test("binary literal int", b"0 + 0b0", Expr::Int(0), None),
        Test(
            "binary literal invalid digits",
            b"0 + 0b2",
            Expr::Int(0),
            Some("invalid digits for radix 2"),
        ),
        Test("octal literal int", b"0 + 0o1", Expr::Int(1), None),
        Test(
            "octal literal int invalid digits",
            b"0 + 0o8",
            Expr::Int(0),
            Some("invalid digits for radix 8"),
        ),
        Test(
            "octal literal int truncated",
            b"0 + 0o",
            Expr::Int(0),
            Some("invalid digits for radix 8"),
        ),
        Test("hex literal int", b"0 + 0x2", Expr::Int(2), None),
        Test(
            "hex literal int invalid digits",
            b"0 + 0x0aq",
            Expr::Int(0),
            Some("invalid digits for radix 16"),
        ),
        Test(
            "hex literal with 'e' not mistaken for float",
            b"0 + 0x00e0",
            Expr::Int(0xe0),
            None,
        ),
        // decimal (rad10) literal is anything that starts with
        // optional sign then a digit.
        Test("rad10 literal int", b"0 + 42", Expr::Int(42), None),
        Test(
            "rad10 with leading + sign",
            b"0 + -42",
            Expr::Int(-42),
            None,
        ),
        Test("rad10 with leading - sign", b"0 + +42", Expr::Int(42), None),
        Test(
            "flag char is string, not (invalid) int",
            b"-x",
            Expr::String("-x".into()),
            None,
        ),
        Test(
            "keyword parameter is string",
            b"--exact",
            Expr::String("--exact".into()),
            None,
        ),
        Test(
            "ranges or relative paths not confused for int",
            b"./a/b",
            Expr::GlobPattern("./a/b".into(), false),
            None,
        ),
        Test(
            "semver data not confused for int",
            b"'1.0.1'",
            Expr::String("1.0.1".into()),
            None,
        ),
    ];

    for test in tests {
        test_int(test.0, test.1, test.2, test.3);
    }
}

#[ignore]
#[test]
pub fn multi_test_parse_number() {
    struct Test<'a>(&'a str, &'a [u8], Expr, Option<&'a str>);

    // use test expression of form '0 + x' to force parse() to parse x as numeric.
    // if expression were just 'x', parse() would try other items that would mask the error we're looking for.
    let tests = vec![
        Test("float decimal", b"0 + 43.5", Expr::Float(43.5), None),
        //Test("float with leading + sign", b"0 + +41.7", Expr::Float(-41.7), None),
        Test(
            "float with leading - sign",
            b"0 + -41.7",
            Expr::Float(-41.7),
            None,
        ),
        Test(
            "float scientific notation",
            b"0 + 3e10",
            Expr::Float(3.00e10),
            None,
        ),
        Test(
            "float decimal literal invalid digits",
            b"0 + .3foo",
            Expr::Int(0),
            Some("invalid digits"),
        ),
        Test(
            "float scientific notation literal invalid digits",
            b"0 + 3e0faa",
            Expr::Int(0),
            Some("invalid digits"),
        ),
        Test(
            // odd that error is unsupportedOperation, but it does fail.
            "decimal literal int 2 leading signs",
            b"0 + --9",
            Expr::Int(0),
            Some("UnsupportedOperation"),
        ),
        //Test(
        //    ".<string> should not be taken as float",
        //    b"abc + .foo",
        //    Expr::String("..".into()),
        //    None,
        //),
    ];

    for test in tests {
        test_int(test.0, test.1, test.2, test.3);
    }
}

#[ignore]
#[test]
fn test_parse_any() {
    let test = b"1..10";
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let block = parse(&mut working_set, None, test, true);

    match (block, working_set.parse_errors.first()) {
        (_, Some(e)) => {
            println!("test: {test:?}, error: {e:#?}");
        }
        (b, None) => {
            println!("test: {test:?}, parse: {b:#?}");
        }
    }
}

#[test]
pub fn parse_int() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let block = parse(&mut working_set, None, b"3", true);

    assert!(working_set.parse_errors.is_empty());
    assert_eq!(block.len(), 1);
    let pipeline = &block.pipelines[0];
    assert_eq!(pipeline.len(), 1);
    let element = &pipeline.elements[0];
    assert!(element.redirection.is_none());
    assert_eq!(element.expr.expr, Expr::Int(3));
}

#[test]
pub fn parse_int_with_underscores() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let block = parse(&mut working_set, None, b"420_69_2023", true);

    assert!(working_set.parse_errors.is_empty());
    assert_eq!(block.len(), 1);
    let pipeline = &block.pipelines[0];
    assert_eq!(pipeline.len(), 1);
    let element = &pipeline.elements[0];
    assert!(element.redirection.is_none());
    assert_eq!(element.expr.expr, Expr::Int(420692023));
}

#[test]
pub fn parse_filesize() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let block = parse(&mut working_set, None, b"95307.27MiB", true);

    assert!(working_set.parse_errors.is_empty());
    assert_eq!(block.len(), 1);
    let pipeline = &block.pipelines[0];
    assert_eq!(pipeline.len(), 1);
    let element = &pipeline.elements[0];
    assert!(element.redirection.is_none());

    let Expr::ValueWithUnit(value) = &element.expr.expr else {
        panic!("should be a ValueWithUnit");
    };

    assert_eq!(value.expr.expr, Expr::Int(99_936_915_947));
    assert_eq!(value.unit.item, Unit::Filesize(FilesizeUnit::B));
}

#[test]
pub fn parse_cell_path() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    working_set.add_variable(
        "foo".to_string().into_bytes(),
        Span::test_data(),
        nu_protocol::Type::record(),
        false,
    );

    let block = parse(&mut working_set, None, b"$foo.bar.baz", true);

    assert!(working_set.parse_errors.is_empty());
    assert_eq!(block.len(), 1);
    let pipeline = &block.pipelines[0];
    assert_eq!(pipeline.len(), 1);
    let element = &pipeline.elements[0];
    assert!(element.redirection.is_none());

    if let Expr::FullCellPath(b) = &element.expr.expr {
        assert!(matches!(b.head.expr, Expr::Var(_)));
        if let [a, b] = &b.tail[..] {
            if let PathMember::String { val, optional, .. } = a {
                assert_eq!(val, "bar");
                assert_eq!(optional, &false);
            } else {
                panic!("wrong type")
            }

            if let PathMember::String { val, optional, .. } = b {
                assert_eq!(val, "baz");
                assert_eq!(optional, &false);
            } else {
                panic!("wrong type")
            }
        } else {
            panic!("cell path tail is unexpected")
        }
    } else {
        panic!("Not a cell path");
    }
}

#[test]
pub fn parse_cell_path_optional() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    working_set.add_variable(
        "foo".to_string().into_bytes(),
        Span::test_data(),
        nu_protocol::Type::record(),
        false,
    );

    let block = parse(&mut working_set, None, b"$foo.bar?.baz", true);

    assert!(working_set.parse_errors.is_empty());

    assert_eq!(block.len(), 1);
    let pipeline = &block.pipelines[0];
    assert_eq!(pipeline.len(), 1);
    let element = &pipeline.elements[0];
    assert!(element.redirection.is_none());

    if let Expr::FullCellPath(b) = &element.expr.expr {
        assert!(matches!(b.head.expr, Expr::Var(_)));
        if let [a, b] = &b.tail[..] {
            if let PathMember::String { val, optional, .. } = a {
                assert_eq!(val, "bar");
                assert_eq!(optional, &true);
            } else {
                panic!("wrong type")
            }

            if let PathMember::String { val, optional, .. } = b {
                assert_eq!(val, "baz");
                assert_eq!(optional, &false);
            } else {
                panic!("wrong type")
            }
        } else {
            panic!("cell path tail is unexpected")
        }
    } else {
        panic!("Not a cell path");
    }
}

#[test]
pub fn parse_binary_with_hex_format() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let block = parse(&mut working_set, None, b"0x[13]", true);

    assert!(working_set.parse_errors.is_empty());
    assert_eq!(block.len(), 1);
    let pipeline = &block.pipelines[0];
    assert_eq!(pipeline.len(), 1);
    let element = &pipeline.elements[0];
    assert!(element.redirection.is_none());
    assert_eq!(element.expr.expr, Expr::Binary(vec![0x13]));
}

#[test]
pub fn parse_binary_with_incomplete_hex_format() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let block = parse(&mut working_set, None, b"0x[3]", true);

    assert!(working_set.parse_errors.is_empty());
    assert_eq!(block.len(), 1);
    let pipeline = &block.pipelines[0];
    assert_eq!(pipeline.len(), 1);
    let element = &pipeline.elements[0];
    assert!(element.redirection.is_none());
    assert_eq!(element.expr.expr, Expr::Binary(vec![0x03]));
}

#[test]
pub fn parse_binary_with_binary_format() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let block = parse(&mut working_set, None, b"0b[1010 1000]", true);

    assert!(working_set.parse_errors.is_empty());
    assert_eq!(block.len(), 1);
    let pipeline = &block.pipelines[0];
    assert_eq!(pipeline.len(), 1);
    let element = &pipeline.elements[0];
    assert!(element.redirection.is_none());
    assert_eq!(element.expr.expr, Expr::Binary(vec![0b10101000]));
}

#[test]
pub fn parse_binary_with_incomplete_binary_format() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let block = parse(&mut working_set, None, b"0b[10]", true);

    assert!(working_set.parse_errors.is_empty());
    assert_eq!(block.len(), 1);
    let pipeline = &block.pipelines[0];
    assert_eq!(pipeline.len(), 1);
    let element = &pipeline.elements[0];
    assert!(element.redirection.is_none());
    assert_eq!(element.expr.expr, Expr::Binary(vec![0b00000010]));
}

#[test]
pub fn parse_binary_with_octal_format() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let block = parse(&mut working_set, None, b"0o[250]", true);

    assert!(working_set.parse_errors.is_empty());
    assert_eq!(block.len(), 1);
    let pipeline = &block.pipelines[0];
    assert_eq!(pipeline.len(), 1);
    let element = &pipeline.elements[0];
    assert!(element.redirection.is_none());
    assert_eq!(element.expr.expr, Expr::Binary(vec![0o250]));
}

#[test]
pub fn parse_binary_with_incomplete_octal_format() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let block = parse(&mut working_set, None, b"0o[2]", true);

    assert!(working_set.parse_errors.is_empty());
    assert_eq!(block.len(), 1);
    let pipeline = &block.pipelines[0];
    assert_eq!(pipeline.len(), 1);
    let element = &pipeline.elements[0];
    assert!(element.redirection.is_none());
    assert_eq!(element.expr.expr, Expr::Binary(vec![0o2]));
}

#[test]
pub fn parse_binary_with_invalid_octal_format() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let block = parse(&mut working_set, None, b"0b[90]", true);

    assert!(working_set.parse_errors.is_empty());
    assert_eq!(block.len(), 1);
    let pipeline = &block.pipelines[0];
    assert_eq!(pipeline.len(), 1);
    let element = &pipeline.elements[0];
    assert!(element.redirection.is_none());
    assert!(!matches!(element.expr.expr, Expr::Binary(_)));
}

#[test]
pub fn parse_binary_with_multi_byte_char() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    // found using fuzzing, Rust can panic if you slice into this string
    let contents = b"0x[\xEF\xBF\xBD]";
    let block = parse(&mut working_set, None, contents, true);

    assert!(working_set.parse_errors.is_empty());
    assert_eq!(block.len(), 1);
    let pipeline = &block.pipelines[0];
    assert_eq!(pipeline.len(), 1);
    let element = &pipeline.elements[0];
    assert!(element.redirection.is_none());
    assert!(!matches!(element.expr.expr, Expr::Binary(_)))
}

#[test]
pub fn parse_call() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let sig = Signature::build("foo").named("--jazz", SyntaxShape::Int, "jazz!!", Some('j'));
    working_set.add_decl(sig.predeclare());

    let block = parse(&mut working_set, None, b"foo", true);

    assert!(working_set.parse_errors.is_empty());
    assert_eq!(block.len(), 1);

    let pipeline = &block.pipelines[0];
    assert_eq!(pipeline.len(), 1);
    let element = &pipeline.elements[0];
    assert!(element.redirection.is_none());

    if let Expr::Call(call) = &element.expr.expr {
        assert_eq!(call.decl_id, DeclId::new(0));
    }
}

#[test]
pub fn parse_call_missing_flag_arg() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let sig = Signature::build("foo").named("jazz", SyntaxShape::Int, "jazz!!", Some('j'));
    working_set.add_decl(sig.predeclare());

    parse(&mut working_set, None, b"foo --jazz", true);
    assert!(matches!(
        working_set.parse_errors.first(),
        Some(ParseError::MissingFlagParam(..))
    ));
}

#[test]
pub fn parse_call_missing_short_flag_arg() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let sig = Signature::build("foo").named("--jazz", SyntaxShape::Int, "jazz!!", Some('j'));
    working_set.add_decl(sig.predeclare());

    parse(&mut working_set, None, b"foo -j", true);
    assert!(matches!(
        working_set.parse_errors.first(),
        Some(ParseError::MissingFlagParam(..))
    ));
}

#[test]
pub fn parse_call_short_flag_batch_arg_allowed() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let sig = Signature::build("foo")
        .named("--jazz", SyntaxShape::Int, "jazz!!", Some('j'))
        .switch("--math", "math!!", Some('m'));
    working_set.add_decl(sig.predeclare());

    let block = parse(&mut working_set, None, b"foo -mj 10", true);

    assert!(working_set.parse_errors.is_empty());
    assert_eq!(block.len(), 1);
    let pipeline = &block.pipelines[0];
    assert_eq!(pipeline.len(), 1);
    let element = &pipeline.elements[0];
    assert!(element.redirection.is_none());

    if let Expr::Call(call) = &element.expr.expr {
        assert_eq!(call.decl_id, DeclId::new(0));
        assert_eq!(call.arguments.len(), 2);
        matches!(call.arguments[0], Argument::Named((_, None, None)));
        matches!(call.arguments[1], Argument::Named((_, None, Some(_))));
    }
}

#[test]
pub fn parse_call_short_flag_batch_arg_disallowed() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let sig = Signature::build("foo")
        .named("--jazz", SyntaxShape::Int, "jazz!!", Some('j'))
        .switch("--math", "math!!", Some('m'));
    working_set.add_decl(sig.predeclare());

    parse(&mut working_set, None, b"foo -jm 10", true);
    assert!(matches!(
        working_set.parse_errors.first(),
        Some(ParseError::OnlyLastFlagInBatchCanTakeArg(..))
    ));
}

#[test]
pub fn parse_call_short_flag_batch_disallow_multiple_args() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let sig = Signature::build("foo")
        .named("--math", SyntaxShape::Int, "math!!", Some('m'))
        .named("--jazz", SyntaxShape::Int, "jazz!!", Some('j'));
    working_set.add_decl(sig.predeclare());

    parse(&mut working_set, None, b"foo -mj 10 20", true);
    assert!(matches!(
        working_set.parse_errors.first(),
        Some(ParseError::OnlyLastFlagInBatchCanTakeArg(..))
    ));
}

#[test]
pub fn parse_call_unknown_shorthand() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let sig = Signature::build("foo").switch("--jazz", "jazz!!", Some('j'));
    working_set.add_decl(sig.predeclare());
    parse(&mut working_set, None, b"foo -mj", true);
    assert!(matches!(
        working_set.parse_errors.first(),
        Some(ParseError::UnknownFlag(..))
    ));
}

#[test]
pub fn parse_call_extra_positional() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let sig = Signature::build("foo").switch("--jazz", "jazz!!", Some('j'));
    working_set.add_decl(sig.predeclare());
    parse(&mut working_set, None, b"foo -j 100", true);
    assert!(matches!(
        working_set.parse_errors.first(),
        Some(ParseError::ExtraPositional(..))
    ));
}

#[test]
pub fn parse_call_missing_req_positional() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let sig = Signature::build("foo").required("jazz", SyntaxShape::Int, "jazz!!");
    working_set.add_decl(sig.predeclare());
    parse(&mut working_set, None, b"foo", true);
    assert!(matches!(
        working_set.parse_errors.first(),
        Some(ParseError::MissingPositional(..))
    ));
}

#[test]
pub fn parse_call_missing_req_flag() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let sig = Signature::build("foo").required_named("--jazz", SyntaxShape::Int, "jazz!!", None);
    working_set.add_decl(sig.predeclare());
    parse(&mut working_set, None, b"foo", true);
    assert!(matches!(
        working_set.parse_errors.first(),
        Some(ParseError::MissingRequiredFlag(..))
    ));
}

#[test]
pub fn parse_attribute_block_check_spans() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let source = br#"
    @foo a 1 2
    @bar b 3 4
    echo baz
    "#;
    let block = parse(&mut working_set, None, source, true);

    // There SHOULD be errors here, we're using nonexistent commands
    assert!(!working_set.parse_errors.is_empty());

    assert_eq!(block.len(), 1);

    let pipeline = &block.pipelines[0];
    assert_eq!(pipeline.len(), 1);
    let element = &pipeline.elements[0];
    assert!(element.redirection.is_none());

    let Expr::AttributeBlock(ab) = &element.expr.expr else {
        panic!("Couldn't parse attribute block");
    };

    assert_eq!(
        working_set.get_span_contents(ab.attributes[0].expr.span),
        b"foo a 1 2"
    );
    assert_eq!(
        working_set.get_span_contents(ab.attributes[1].expr.span),
        b"bar b 3 4"
    );
    assert_eq!(working_set.get_span_contents(ab.item.span), b"echo baz");
}

#[test]
pub fn parse_attributes_check_values() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    working_set.add_decl(Box::new(Def));
    working_set.add_decl(Box::new(AttrEcho));

    let source = br#"
    @echo "hello world"
    @echo 42
    def foo [] {}
    "#;
    let _ = parse(&mut working_set, None, source, false);

    assert!(working_set.parse_errors.is_empty());

    let decl_id = working_set.find_decl(b"foo").unwrap();
    let cmd = working_set.get_decl(decl_id);
    let attributes = cmd.attributes();

    let (name, val) = &attributes[0];
    assert_eq!(name, "echo");
    assert_eq!(val.as_str(), Ok("hello world"));

    let (name, val) = &attributes[1];
    assert_eq!(name, "echo");
    assert_eq!(val.as_int(), Ok(42));
}

#[test]
pub fn parse_attributes_alias() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    working_set.add_decl(Box::new(Def));
    working_set.add_decl(Box::new(Alias));
    working_set.add_decl(Box::new(AttrEcho));

    let source = br#"
    alias "attr test" = attr echo

    @test null
    def foo [] {}
    "#;
    let _ = parse(&mut working_set, None, source, false);

    assert!(working_set.parse_errors.is_empty());

    let decl_id = working_set.find_decl(b"foo").unwrap();
    let cmd = working_set.get_decl(decl_id);
    let attributes = cmd.attributes();

    let (name, val) = &attributes[0];
    assert_eq!(name, "test");
    assert!(val.is_nothing());
}

#[test]
pub fn parse_attributes_external_alias() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    working_set.add_decl(Box::new(Def));
    working_set.add_decl(Box::new(Alias));
    working_set.add_decl(Box::new(AttrEcho));

    let source = br#"
    alias "attr test" = ^echo

    @test null
    def foo [] {}
    "#;
    let _ = parse(&mut working_set, None, source, false);

    assert!(!working_set.parse_errors.is_empty());

    let ParseError::LabeledError(shell_error, parse_error, _span) = &working_set.parse_errors[0]
    else {
        panic!("Expected LabeledError");
    };

    assert!(shell_error.contains("nu::shell::not_a_const_command"));
    assert!(parse_error.contains("Encountered error during parse-time evaluation"));
}

#[test]
pub fn parse_if_in_const_expression() {
    // https://github.com/nushell/nushell/issues/15321
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    working_set.add_decl(Box::new(Const));
    working_set.add_decl(Box::new(Def));
    working_set.add_decl(Box::new(IfMocked));

    let source = b"const foo = if t";
    let _ = parse(&mut working_set, None, source, false);

    assert!(!working_set.parse_errors.is_empty());
    let ParseError::MissingPositional(error, _, _) = &working_set.parse_errors[0] else {
        panic!("Expected MissingPositional");
    };

    assert!(error.contains("cond"));

    working_set.parse_errors = Vec::new();
    let source = b"def a [n= (if ]";
    let _ = parse(&mut working_set, None, source, false);

    assert!(!working_set.parse_errors.is_empty());
    let ParseError::UnexpectedEof(error, _) = &working_set.parse_errors[0] else {
        panic!("Expected UnexpectedEof");
    };

    assert!(error.contains(")"));
}

fn test_external_call(input: &str, tag: &str, f: impl FnOnce(&Expression, &[ExternalArgument])) {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);
    let block = parse(&mut working_set, None, input.as_bytes(), true);
    assert!(
        working_set.parse_errors.is_empty(),
        "{tag}: errors: {:?}",
        working_set.parse_errors
    );

    let pipeline = &block.pipelines[0];
    assert_eq!(1, pipeline.len());
    let element = &pipeline.elements[0];
    match &element.expr.expr {
        Expr::ExternalCall(name, args) => f(name, args),
        other => {
            panic!("{tag}: Unexpected expression in pipeline: {other:?}");
        }
    }
}

fn check_external_call_interpolation(
    tag: &str,
    subexpr_count: usize,
    quoted: bool,
    expr: &Expression,
) -> bool {
    match &expr.expr {
        Expr::StringInterpolation(exprs) => {
            assert!(quoted, "{tag}: quoted");
            assert_eq!(expr.ty, Type::String, "{tag}: expr.ty");
            assert_eq!(subexpr_count, exprs.len(), "{tag}: subexpr_count");
            true
        }
        Expr::GlobInterpolation(exprs, is_quoted) => {
            assert_eq!(quoted, *is_quoted, "{tag}: quoted");
            assert_eq!(expr.ty, Type::Glob, "{tag}: expr.ty");
            assert_eq!(subexpr_count, exprs.len(), "{tag}: subexpr_count");
            true
        }
        _ => false,
    }
}

#[rstest]
#[case("foo-external-call", "foo-external-call", "bare word")]
#[case("^foo-external-call", "foo-external-call", "bare word with caret")]
#[case(
    "foo/external-call",
    "foo/external-call",
    "bare word with forward slash"
)]
#[case(
    "^foo/external-call",
    "foo/external-call",
    "bare word with forward slash and caret"
)]
#[case(r"foo\external-call", r"foo\external-call", "bare word with backslash")]
#[case(
    r"^foo\external-call",
    r"foo\external-call",
    "bare word with backslash and caret"
)]
#[case("`foo external call`", "foo external call", "backtick quote")]
#[case(
    "^`foo external call`",
    "foo external call",
    "backtick quote with caret"
)]
#[case(
    "`foo/external call`",
    "foo/external call",
    "backtick quote with forward slash"
)]
#[case(
    "^`foo/external call`",
    "foo/external call",
    "backtick quote with forward slash and caret"
)]
#[case(
    r"`foo\external call`",
    r"foo\external call",
    "backtick quote with backslash"
)]
#[case(
    r"^`foo\external call`",
    r"foo\external call",
    "backtick quote with backslash and caret"
)]
pub fn test_external_call_head_glob(
    #[case] input: &str,
    #[case] expected: &str,
    #[case] tag: &str,
) {
    test_external_call(input, tag, |name, args| {
        match &name.expr {
            Expr::GlobPattern(string, is_quoted) => {
                assert_eq!(expected, string, "{tag}: incorrect name");
                assert!(!*is_quoted);
            }
            other => {
                panic!("{tag}: Unexpected expression in command name position: {other:?}");
            }
        }
        assert_eq!(0, args.len());
    })
}

#[rstest]
#[case(
    r##"^r#'foo-external-call'#"##,
    "foo-external-call",
    "raw string with caret"
)]
#[case(
    r##"^r#'foo/external-call'#"##,
    "foo/external-call",
    "raw string with forward slash and caret"
)]
#[case(
    r##"^r#'foo\external-call'#"##,
    r"foo\external-call",
    "raw string with backslash and caret"
)]
pub fn test_external_call_head_raw_string(
    #[case] input: &str,
    #[case] expected: &str,
    #[case] tag: &str,
) {
    test_external_call(input, tag, |name, args| {
        match &name.expr {
            Expr::RawString(string) => {
                assert_eq!(expected, string, "{tag}: incorrect name");
            }
            other => {
                panic!("{tag}: Unexpected expression in command name position: {other:?}");
            }
        }
        assert_eq!(0, args.len());
    })
}

#[rstest]
#[case("^'foo external call'", "foo external call", "single quote with caret")]
#[case(
    "^'foo/external call'",
    "foo/external call",
    "single quote with forward slash and caret"
)]
#[case(
    r"^'foo\external call'",
    r"foo\external call",
    "single quote with backslash and caret"
)]
#[case(
    r#"^"foo external call""#,
    r#"foo external call"#,
    "double quote with caret"
)]
#[case(
    r#"^"foo/external call""#,
    r#"foo/external call"#,
    "double quote with forward slash and caret"
)]
#[case(
    r#"^"foo\\external call""#,
    r#"foo\external call"#,
    "double quote with backslash and caret"
)]
pub fn test_external_call_head_string(
    #[case] input: &str,
    #[case] expected: &str,
    #[case] tag: &str,
) {
    test_external_call(input, tag, |name, args| {
        match &name.expr {
            Expr::String(string) => {
                assert_eq!(expected, string);
            }
            other => {
                panic!("{tag}: Unexpected expression in command name position: {other:?}");
            }
        }
        assert_eq!(0, args.len());
    })
}

#[rstest]
#[case(r"~/.foo/(1)", 2, false, "unquoted interpolated string")]
#[case(
    r"~\.foo(2)\(1)",
    4,
    false,
    "unquoted interpolated string with backslash"
)]
#[case(r"^~/.foo/(1)", 2, false, "unquoted interpolated string with caret")]
#[case(r#"^$"~/.foo/(1)""#, 2, true, "quoted interpolated string with caret")]
pub fn test_external_call_head_interpolated_string(
    #[case] input: &str,
    #[case] subexpr_count: usize,
    #[case] quoted: bool,
    #[case] tag: &str,
) {
    test_external_call(input, tag, |name, args| {
        if !check_external_call_interpolation(tag, subexpr_count, quoted, name) {
            panic!("{tag}: Unexpected expression in command name position: {name:?}");
        }
        assert_eq!(0, args.len());
    })
}

#[rstest]
#[case("^foo foo-external-call", "foo-external-call", "bare word")]
#[case(
    "^foo foo/external-call",
    "foo/external-call",
    "bare word with forward slash"
)]
#[case(
    r"^foo foo\external-call",
    r"foo\external-call",
    "bare word with backslash"
)]
#[case(
    "^foo `foo external call`",
    "foo external call",
    "backtick quote with caret"
)]
#[case(
    "^foo `foo/external call`",
    "foo/external call",
    "backtick quote with forward slash"
)]
#[case(
    r"^foo `foo\external call`",
    r"foo\external call",
    "backtick quote with backslash"
)]
#[case(
    r#"^foo --flag="value""#,
    r#"--flag=value"#,
    "flag value with double quote"
)]
#[case(
    r#"^foo --flag='value'"#,
    r#"--flag=value"#,
    "flag value with single quote"
)]
#[case(
    r#"^foo {a:1,b:'c',c:'d'}"#,
    r#"{a:1,b:c,c:d}"#,
    "value with many inner single quotes"
)]
#[case(
    r#"^foo {a:1,b:"c",c:"d"}"#,
    r#"{a:1,b:c,c:d}"#,
    "value with many double quotes"
)]
#[case(
    r#"^foo {a:1,b:'c',c:"d"}"#,
    r#"{a:1,b:c,c:d}"#,
    "value with single quote and double quote"
)]
#[case(
    r#"^foo `hello world`"#,
    r#"hello world"#,
    "value is surrounded by backtick quote"
)]
#[case(
    r#"^foo `"hello world"`"#,
    "\"hello world\"",
    "value is surrounded by backtick quote, with inner double quote"
)]
#[case(
    r#"^foo `'hello world'`"#,
    "'hello world'",
    "value is surrounded by backtick quote, with inner single quote"
)]
pub fn test_external_call_arg_glob(#[case] input: &str, #[case] expected: &str, #[case] tag: &str) {
    test_external_call(input, tag, |name, args| {
        match &name.expr {
            Expr::GlobPattern(string, _) => {
                assert_eq!("foo", string, "{tag}: incorrect name");
            }
            other => {
                panic!("{tag}: Unexpected expression in command name position: {other:?}");
            }
        }
        assert_eq!(1, args.len());
        match &args[0] {
            ExternalArgument::Regular(expr) => match &expr.expr {
                Expr::GlobPattern(string, is_quoted) => {
                    assert_eq!(expected, string, "{tag}: incorrect arg");
                    assert!(!*is_quoted);
                }
                other => {
                    panic!("Unexpected expression in command arg position: {other:?}")
                }
            },
            other @ ExternalArgument::Spread(..) => {
                panic!("Unexpected external spread argument in command arg position: {other:?}")
            }
        }
    })
}

#[rstest]
#[case(r##"^foo r#'foo-external-call'#"##, "foo-external-call", "raw string")]
#[case(
    r##"^foo r#'foo/external-call'#"##,
    "foo/external-call",
    "raw string with forward slash"
)]
#[case(
    r##"^foo r#'foo\external-call'#"##,
    r"foo\external-call",
    "raw string with backslash"
)]
pub fn test_external_call_arg_raw_string(
    #[case] input: &str,
    #[case] expected: &str,
    #[case] tag: &str,
) {
    test_external_call(input, tag, |name, args| {
        match &name.expr {
            Expr::GlobPattern(string, _) => {
                assert_eq!("foo", string, "{tag}: incorrect name");
            }
            other => {
                panic!("{tag}: Unexpected expression in command name position: {other:?}");
            }
        }
        assert_eq!(1, args.len());
        match &args[0] {
            ExternalArgument::Regular(expr) => match &expr.expr {
                Expr::RawString(string) => {
                    assert_eq!(expected, string, "{tag}: incorrect arg");
                }
                other => {
                    panic!("Unexpected expression in command arg position: {other:?}")
                }
            },
            other @ ExternalArgument::Spread(..) => {
                panic!("Unexpected external spread argument in command arg position: {other:?}")
            }
        }
    })
}

#[rstest]
#[case("^foo 'foo external call'", "foo external call", "single quote")]
#[case(
    "^foo 'foo/external call'",
    "foo/external call",
    "single quote with forward slash"
)]
#[case(
    r"^foo 'foo\external call'",
    r"foo\external call",
    "single quote with backslash"
)]
#[case(r#"^foo "foo external call""#, r#"foo external call"#, "double quote")]
#[case(
    r#"^foo "foo/external call""#,
    r#"foo/external call"#,
    "double quote with forward slash"
)]
#[case(
    r#"^foo "foo\\external call""#,
    r#"foo\external call"#,
    "double quote with backslash"
)]
pub fn test_external_call_arg_string(
    #[case] input: &str,
    #[case] expected: &str,
    #[case] tag: &str,
) {
    test_external_call(input, tag, |name, args| {
        match &name.expr {
            Expr::GlobPattern(string, _) => {
                assert_eq!("foo", string, "{tag}: incorrect name");
            }
            other => {
                panic!("{tag}: Unexpected expression in command name position: {other:?}");
            }
        }
        assert_eq!(1, args.len());
        match &args[0] {
            ExternalArgument::Regular(expr) => match &expr.expr {
                Expr::String(string) => {
                    assert_eq!(expected, string, "{tag}: incorrect arg");
                }
                other => {
                    panic!("{tag}: Unexpected expression in command arg position: {other:?}")
                }
            },
            other @ ExternalArgument::Spread(..) => {
                panic!(
                    "{tag}: Unexpected external spread argument in command arg position: {other:?}"
                )
            }
        }
    })
}

#[rstest]
#[case(r"^foo ~/.foo/(1)", 2, false, "unquoted interpolated string")]
#[case(r#"^foo $"~/.foo/(1)""#, 2, true, "quoted interpolated string")]
pub fn test_external_call_arg_interpolated_string(
    #[case] input: &str,
    #[case] subexpr_count: usize,
    #[case] quoted: bool,
    #[case] tag: &str,
) {
    test_external_call(input, tag, |name, args| {
        match &name.expr {
            Expr::GlobPattern(string, _) => {
                assert_eq!("foo", string, "{tag}: incorrect name");
            }
            other => {
                panic!("{tag}: Unexpected expression in command name position: {other:?}");
            }
        }
        assert_eq!(1, args.len());
        match &args[0] {
            ExternalArgument::Regular(expr) => {
                if !check_external_call_interpolation(tag, subexpr_count, quoted, expr) {
                    panic!("Unexpected expression in command arg position: {expr:?}")
                }
            }
            other @ ExternalArgument::Spread(..) => {
                panic!("Unexpected external spread argument in command arg position: {other:?}")
            }
        }
    })
}

#[test]
fn test_external_call_argument_spread() {
    let input = r"^foo ...[a b c]";
    let tag = "spread";

    test_external_call(input, tag, |name, args| {
        match &name.expr {
            Expr::GlobPattern(string, _) => {
                assert_eq!("foo", string, "incorrect name");
            }
            other => {
                panic!("Unexpected expression in command name position: {other:?}");
            }
        }
        assert_eq!(1, args.len());
        match &args[0] {
            ExternalArgument::Spread(expr) => match &expr.expr {
                Expr::List(items) => {
                    assert_eq!(3, items.len());
                    // that's good enough, don't really need to go so deep into it...
                }
                other => {
                    panic!("Unexpected expression in command arg position: {other:?}")
                }
            },
            other @ ExternalArgument::Regular(..) => {
                panic!("Unexpected external regular argument in command arg position: {other:?}")
            }
        }
    })
}

#[test]
fn test_nothing_comparison_eq() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);
    let block = parse(&mut working_set, None, b"2 == null", true);

    assert!(working_set.parse_errors.is_empty());
    assert_eq!(block.len(), 1);

    let pipeline = &block.pipelines[0];
    assert_eq!(pipeline.len(), 1);
    let element = &pipeline.elements[0];
    assert!(element.redirection.is_none());
    assert!(matches!(&element.expr.expr, Expr::BinaryOp(..)));
}

#[rstest]
#[case(b"let a o> file = 1")]
#[case(b"mut a o> file = 1")]
fn test_redirection_inside_letmut_no_panic(#[case] phase: &[u8]) {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);
    working_set.add_decl(Box::new(Let));
    working_set.add_decl(Box::new(Mut));

    parse(&mut working_set, None, phase, true);
}

#[rstest]
#[case(b"let a = 1 err> /dev/null")]
#[case(b"let a = 1 out> /dev/null")]
#[case(b"let a = 1 out+err> /dev/null")]
#[case(b"mut a = 1 err> /dev/null")]
#[case(b"mut a = 1 out> /dev/null")]
#[case(b"mut a = 1 out+err> /dev/null")]
fn test_redirection_with_letmut(#[case] phase: &[u8]) {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);
    working_set.add_decl(Box::new(Let));
    working_set.add_decl(Box::new(Mut));

    let block = parse(&mut working_set, None, phase, true);
    assert!(
        working_set.parse_errors.is_empty(),
        "parse errors: {:?}",
        working_set.parse_errors
    );
    assert_eq!(1, block.pipelines[0].elements.len());

    let element = &block.pipelines[0].elements[0];
    assert!(element.redirection.is_none()); // it should be in the let block, not here

    if let Expr::Call(call) = &element.expr.expr {
        let arg = call.positional_nth(1).expect("no positional args");
        let block_id = arg.as_block().expect("arg 1 is not a block");
        let block = working_set.get_block(block_id);
        let inner_element = &block.pipelines[0].elements[0];
        assert!(inner_element.redirection.is_some());
    } else {
        panic!("expected Call: {:?}", block.pipelines[0].elements[0])
    }
}

#[rstest]
#[case(b"o>")]
#[case(b"o>>")]
#[case(b"e>")]
#[case(b"e>>")]
#[case(b"o+e>")]
#[case(b"o+e>>")]
#[case(b"e>|")]
#[case(b"o+e>|")]
#[case(b"|o>")]
#[case(b"|o>>")]
#[case(b"|e>")]
#[case(b"|e>>")]
#[case(b"|o+e>")]
#[case(b"|o+e>>")]
#[case(b"|e>|")]
#[case(b"|o+e>|")]
#[case(b"e> file")]
#[case(b"e>> file")]
#[case(b"o> file")]
#[case(b"o>> file")]
#[case(b"o+e> file")]
#[case(b"o+e>> file")]
#[case(b"|e> file")]
#[case(b"|e>> file")]
#[case(b"|o> file")]
#[case(b"|o>> file")]
#[case(b"|o+e> file")]
#[case(b"|o+e>> file")]
fn test_redirecting_nothing(#[case] text: &[u8]) {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);
    let _ = parse(&mut working_set, None, text, true);
    assert!(matches!(
        working_set.parse_errors.first(),
        Some(ParseError::UnexpectedRedirection { .. })
    ));
}

#[test]
fn test_nothing_comparison_neq() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);
    let block = parse(&mut working_set, None, b"2 != null", true);

    assert!(working_set.parse_errors.is_empty());
    assert_eq!(block.len(), 1);

    let pipeline = &block.pipelines[0];
    assert_eq!(pipeline.len(), 1);
    let element = &pipeline.elements[0];
    assert!(element.redirection.is_none());
    assert!(matches!(&element.expr.expr, Expr::BinaryOp(..)));
}

mod string {
    use super::*;

    #[test]
    pub fn parse_string() {
        let engine_state = EngineState::new();
        let mut working_set = StateWorkingSet::new(&engine_state);

        let block = parse(&mut working_set, None, b"\"hello nushell\"", true);

        assert!(working_set.parse_errors.is_empty());
        assert_eq!(block.len(), 1);
        let pipeline = &block.pipelines[0];
        assert_eq!(pipeline.len(), 1);
        let element = &pipeline.elements[0];
        assert!(element.redirection.is_none());
        assert_eq!(element.expr.expr, Expr::String("hello nushell".to_string()))
    }

    mod interpolation {
        use nu_protocol::Span;

        use super::*;

        #[test]
        pub fn parse_string_interpolation() {
            let engine_state = EngineState::new();
            let mut working_set = StateWorkingSet::new(&engine_state);

            let block = parse(&mut working_set, None, b"$\"hello (39 + 3)\"", true);

            assert!(working_set.parse_errors.is_empty());
            assert_eq!(block.len(), 1);

            let pipeline = &block.pipelines[0];
            assert_eq!(pipeline.len(), 1);
            let element = &pipeline.elements[0];
            assert!(element.redirection.is_none());

            let subexprs: Vec<&Expr> = match &element.expr.expr {
                Expr::StringInterpolation(expressions) => {
                    expressions.iter().map(|e| &e.expr).collect()
                }
                _ => panic!("Expected an `Expr::StringInterpolation`"),
            };

            assert_eq!(subexprs.len(), 2);

            assert_eq!(subexprs[0], &Expr::String("hello ".to_string()));

            assert!(matches!(subexprs[1], &Expr::FullCellPath(..)));
        }

        #[test]
        pub fn parse_string_interpolation_escaped_parenthesis() {
            let engine_state = EngineState::new();
            let mut working_set = StateWorkingSet::new(&engine_state);

            let block = parse(&mut working_set, None, b"$\"hello \\(39 + 3)\"", true);

            assert!(working_set.parse_errors.is_empty());

            assert_eq!(block.len(), 1);
            let pipeline = &block.pipelines[0];
            assert_eq!(pipeline.len(), 1);
            let element = &pipeline.elements[0];
            assert!(element.redirection.is_none());

            let subexprs: Vec<&Expr> = match &element.expr.expr {
                Expr::StringInterpolation(expressions) => {
                    expressions.iter().map(|e| &e.expr).collect()
                }
                _ => panic!("Expected an `Expr::StringInterpolation`"),
            };

            assert_eq!(subexprs.len(), 1);

            assert_eq!(subexprs[0], &Expr::String("hello (39 + 3)".to_string()));
        }

        #[test]
        pub fn parse_string_interpolation_escaped_backslash_before_parenthesis() {
            let engine_state = EngineState::new();
            let mut working_set = StateWorkingSet::new(&engine_state);

            let block = parse(&mut working_set, None, b"$\"hello \\\\(39 + 3)\"", true);

            assert!(working_set.parse_errors.is_empty());

            assert_eq!(block.len(), 1);
            let pipeline = &block.pipelines[0];
            assert_eq!(pipeline.len(), 1);
            let element = &pipeline.elements[0];
            assert!(element.redirection.is_none());

            let subexprs: Vec<&Expr> = match &element.expr.expr {
                Expr::StringInterpolation(expressions) => {
                    expressions.iter().map(|e| &e.expr).collect()
                }
                _ => panic!("Expected an `Expr::StringInterpolation`"),
            };

            assert_eq!(subexprs.len(), 2);

            assert_eq!(subexprs[0], &Expr::String("hello \\".to_string()));

            assert!(matches!(subexprs[1], &Expr::FullCellPath(..)));
        }

        #[test]
        pub fn parse_string_interpolation_backslash_count_reset_by_expression() {
            let engine_state = EngineState::new();
            let mut working_set = StateWorkingSet::new(&engine_state);

            let block = parse(&mut working_set, None, b"$\"\\(1 + 3)\\(7 - 5)\"", true);

            assert!(working_set.parse_errors.is_empty());

            assert_eq!(block.len(), 1);
            let pipeline = &block.pipelines[0];
            assert_eq!(pipeline.len(), 1);
            let element = &pipeline.elements[0];
            assert!(element.redirection.is_none());

            let subexprs: Vec<&Expr> = match &element.expr.expr {
                Expr::StringInterpolation(expressions) => {
                    expressions.iter().map(|e| &e.expr).collect()
                }
                _ => panic!("Expected an `Expr::StringInterpolation`"),
            };

            assert_eq!(subexprs.len(), 1);
            assert_eq!(subexprs[0], &Expr::String("(1 + 3)(7 - 5)".to_string()));
        }

        #[test]
        pub fn parse_string_interpolation_bare() {
            let engine_state = EngineState::new();
            let mut working_set = StateWorkingSet::new(&engine_state);

            let block = parse(
                &mut working_set,
                None,
                b"\"\" ++ foo(1 + 3)bar(7 - 5)",
                true,
            );

            assert!(working_set.parse_errors.is_empty());

            assert_eq!(block.len(), 1);
            let pipeline = &block.pipelines[0];
            assert_eq!(pipeline.len(), 1);
            let element = &pipeline.elements[0];
            assert!(element.redirection.is_none());

            let subexprs: Vec<&Expr> = match &element.expr.expr {
                Expr::BinaryOp(_, _, rhs) => match &rhs.expr {
                    Expr::StringInterpolation(expressions) => {
                        expressions.iter().map(|e| &e.expr).collect()
                    }
                    _ => panic!("Expected an `Expr::StringInterpolation`"),
                },
                _ => panic!("Expected an `Expr::BinaryOp`"),
            };

            assert_eq!(subexprs.len(), 4);

            assert_eq!(subexprs[0], &Expr::String("foo".to_string()));
            assert!(matches!(subexprs[1], &Expr::FullCellPath(..)));
            assert_eq!(subexprs[2], &Expr::String("bar".to_string()));
            assert!(matches!(subexprs[3], &Expr::FullCellPath(..)));
        }

        #[test]
        pub fn parse_string_interpolation_bare_starting_subexpr() {
            let engine_state = EngineState::new();
            let mut working_set = StateWorkingSet::new(&engine_state);

            let block = parse(
                &mut working_set,
                None,
                b"\"\" ++ (1 + 3)foo(7 - 5)bar",
                true,
            );

            assert!(working_set.parse_errors.is_empty());

            assert_eq!(block.len(), 1);
            let pipeline = &block.pipelines[0];
            assert_eq!(pipeline.len(), 1);
            let element = &pipeline.elements[0];
            assert!(element.redirection.is_none());

            let subexprs: Vec<&Expr> = match &element.expr.expr {
                Expr::BinaryOp(_, _, rhs) => match &rhs.expr {
                    Expr::StringInterpolation(expressions) => {
                        expressions.iter().map(|e| &e.expr).collect()
                    }
                    _ => panic!("Expected an `Expr::StringInterpolation`"),
                },
                _ => panic!("Expected an `Expr::BinaryOp`"),
            };

            assert_eq!(subexprs.len(), 4);

            assert!(matches!(subexprs[0], &Expr::FullCellPath(..)));
            assert_eq!(subexprs[1], &Expr::String("foo".to_string()));
            assert!(matches!(subexprs[2], &Expr::FullCellPath(..)));
            assert_eq!(subexprs[3], &Expr::String("bar".to_string()));
        }

        #[test]
        pub fn parse_string_interpolation_bare_starting_subexpr_external_arg() {
            let engine_state = EngineState::new();
            let mut working_set = StateWorkingSet::new(&engine_state);

            let block = parse(&mut working_set, None, b"^echo ($nu.home-path)/path", true);

            assert!(working_set.parse_errors.is_empty());

            assert_eq!(block.len(), 1);
            let pipeline = &block.pipelines[0];
            assert_eq!(pipeline.len(), 1);
            let element = &pipeline.elements[0];
            assert!(element.redirection.is_none());

            let subexprs: Vec<&Expr> = match &element.expr.expr {
                Expr::ExternalCall(_, args) => match &args[0] {
                    ExternalArgument::Regular(expression) => match &expression.expr {
                        Expr::StringInterpolation(expressions) => {
                            expressions.iter().map(|e| &e.expr).collect()
                        }
                        _ => panic!("Expected an `ExternalArgument::Regular`"),
                    },
                    _ => panic!("Expected an `Expr::StringInterpolation`"),
                },
                _ => panic!("Expected an `Expr::BinaryOp`"),
            };

            assert_eq!(subexprs.len(), 2);

            assert!(matches!(subexprs[0], &Expr::FullCellPath(..)));
            assert_eq!(subexprs[1], &Expr::String("/path".to_string()));
        }

        #[test]
        pub fn parse_nested_expressions() {
            let engine_state = EngineState::new();
            let mut working_set = StateWorkingSet::new(&engine_state);

            working_set.add_variable(
                "foo".to_string().into_bytes(),
                Span::new(0, 0),
                nu_protocol::Type::CellPath,
                false,
            );

            parse(
                &mut working_set,
                None,
                br#"
                $"(($foo))"
                "#,
                true,
            );

            assert!(working_set.parse_errors.is_empty());
        }

        #[test]
        pub fn parse_path_expression() {
            let engine_state = EngineState::new();
            let mut working_set = StateWorkingSet::new(&engine_state);

            working_set.add_variable(
                "foo".to_string().into_bytes(),
                Span::new(0, 0),
                nu_protocol::Type::CellPath,
                false,
            );

            parse(
                &mut working_set,
                None,
                br#"
                $"Hello ($foo.bar)"
                "#,
                true,
            );

            assert!(working_set.parse_errors.is_empty());
        }
    }

    #[test]
    fn parse_raw_string_as_external_argument() {
        let engine_state = EngineState::new();
        let mut working_set = StateWorkingSet::new(&engine_state);

        let block = parse(&mut working_set, None, b"^echo r#'text'#", true);

        assert!(working_set.parse_errors.is_empty());
        assert_eq!(block.len(), 1);
        let pipeline = &block.pipelines[0];
        assert_eq!(pipeline.len(), 1);
        let element = &pipeline.elements[0];
        assert!(element.redirection.is_none());
        if let Expr::ExternalCall(_, args) = &element.expr.expr {
            if let [ExternalArgument::Regular(expr)] = args.as_ref() {
                assert_eq!(expr.expr, Expr::RawString("text".into()));
                return;
            }
        }
        panic!("wrong expression: {:?}", element.expr.expr)
    }
}

#[rstest]
#[case(b"let a = }")]
#[case(b"mut a = }")]
#[case(b"let a = | }")]
#[case(b"mut a = | }")]
fn test_semi_open_brace(#[case] phrase: &[u8]) {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);
    // this should not panic
    let _block = parse(&mut working_set, None, phrase, true);
}

mod range {
    use super::*;
    use nu_protocol::ast::{RangeInclusion, RangeOperator};

    #[rstest]
    #[case(b"0..10", RangeInclusion::Inclusive, "inclusive")]
    #[case(b"0..=10", RangeInclusion::Inclusive, "=inclusive")]
    #[case(b"0..<10", RangeInclusion::RightExclusive, "exclusive")]
    #[case(b"10..0", RangeInclusion::Inclusive, "reverse inclusive")]
    #[case(b"10..=0", RangeInclusion::Inclusive, "reverse =inclusive")]
    #[case(
        b"(3 - 3)..<(8 + 2)",
        RangeInclusion::RightExclusive,
        "subexpression exclusive"
    )]
    #[case(
        b"(3 - 3)..(8 + 2)",
        RangeInclusion::Inclusive,
        "subexpression inclusive"
    )]
    #[case(
        b"(3 - 3)..=(8 + 2)",
        RangeInclusion::Inclusive,
        "subexpression =inclusive"
    )]
    #[case(b"-10..-3", RangeInclusion::Inclusive, "negative inclusive")]
    #[case(b"-10..=-3", RangeInclusion::Inclusive, "negative =inclusive")]
    #[case(b"-10..<-3", RangeInclusion::RightExclusive, "negative exclusive")]

    fn parse_bounded_range(
        #[case] phrase: &[u8],
        #[case] inclusion: RangeInclusion,
        #[case] tag: &str,
    ) {
        let engine_state = EngineState::new();
        let mut working_set = StateWorkingSet::new(&engine_state);

        let block = parse(&mut working_set, None, phrase, true);

        assert!(working_set.parse_errors.is_empty());
        assert_eq!(block.len(), 1, "{tag}: block length");

        let pipeline = &block.pipelines[0];
        assert_eq!(pipeline.len(), 1, "{tag}: expression length");
        let element = &pipeline.elements[0];
        assert!(element.redirection.is_none());
        if let Expr::Range(range) = &element.expr.expr {
            if let Range {
                from: Some(_),
                next: None,
                to: Some(_),
                operator:
                    RangeOperator {
                        inclusion: the_inclusion,
                        ..
                    },
            } = range.as_ref()
            {
                assert_eq!(
                    *the_inclusion, inclusion,
                    "{tag}: wrong RangeInclusion {the_inclusion:?}"
                );
            } else {
                panic!("{tag}: expression mismatch.")
            }
        } else {
            panic!("{tag}: expression mismatch.")
        };
    }

    #[rstest]
    #[case(
        b"let a = 2; $a..10",
        RangeInclusion::Inclusive,
        "variable start inclusive"
    )]
    #[case(
        b"let a = 2; $a..=10",
        RangeInclusion::Inclusive,
        "variable start =inclusive"
    )]
    #[case(
        b"let a = 2; $a..<($a + 10)",
        RangeInclusion::RightExclusive,
        "subexpression variable exclusive"
    )]
    fn parse_variable_range(
        #[case] phrase: &[u8],
        #[case] inclusion: RangeInclusion,
        #[case] tag: &str,
    ) {
        let engine_state = EngineState::new();
        let mut working_set = StateWorkingSet::new(&engine_state);

        working_set.add_decl(Box::new(Let));

        let block = parse(&mut working_set, None, phrase, true);

        assert!(working_set.parse_errors.is_empty());
        assert_eq!(block.len(), 2, "{tag} block len 2");

        let pipeline = &block.pipelines[1];
        assert_eq!(pipeline.len(), 1, "{tag}: expression length 1");
        let element = &pipeline.elements[0];
        assert!(element.redirection.is_none());
        if let Expr::Range(range) = &element.expr.expr {
            if let Range {
                from: Some(_),
                next: None,
                to: Some(_),
                operator:
                    RangeOperator {
                        inclusion: the_inclusion,
                        ..
                    },
            } = range.as_ref()
            {
                assert_eq!(
                    *the_inclusion, inclusion,
                    "{tag}: wrong RangeInclusion {the_inclusion:?}"
                );
            } else {
                panic!("{tag}: expression mismatch.")
            }
        } else {
            panic!("{tag}: expression mismatch.")
        };
    }

    #[rstest]
    #[case(b"0..", RangeInclusion::Inclusive, "right unbounded")]
    #[case(b"0..=", RangeInclusion::Inclusive, "right unbounded =inclusive")]
    #[case(b"0..<", RangeInclusion::RightExclusive, "right unbounded")]

    fn parse_right_unbounded_range(
        #[case] phrase: &[u8],
        #[case] inclusion: RangeInclusion,
        #[case] tag: &str,
    ) {
        let engine_state = EngineState::new();
        let mut working_set = StateWorkingSet::new(&engine_state);

        let block = parse(&mut working_set, None, phrase, true);

        assert!(working_set.parse_errors.is_empty());
        assert_eq!(block.len(), 1, "{tag}: block len 1");

        let pipeline = &block.pipelines[0];
        assert_eq!(pipeline.len(), 1, "{tag}: expression length");
        let element = &pipeline.elements[0];
        assert!(element.redirection.is_none());
        if let Expr::Range(range) = &element.expr.expr {
            if let Range {
                from: Some(_),
                next: None,
                to: None,
                operator:
                    RangeOperator {
                        inclusion: the_inclusion,
                        ..
                    },
            } = range.as_ref()
            {
                assert_eq!(
                    *the_inclusion, inclusion,
                    "{tag}: wrong RangeInclusion {the_inclusion:?}"
                );
            } else {
                panic!("{tag}: expression mismatch.")
            }
        } else {
            panic!("{tag}: expression mismatch.")
        };
    }

    #[rstest]
    #[case(b"..10", RangeInclusion::Inclusive, "left unbounded inclusive")]
    #[case(b"..=10", RangeInclusion::Inclusive, "left unbounded =inclusive")]
    #[case(b"..<10", RangeInclusion::RightExclusive, "left unbounded exclusive")]

    fn parse_left_unbounded_range(
        #[case] phrase: &[u8],
        #[case] inclusion: RangeInclusion,
        #[case] tag: &str,
    ) {
        let engine_state = EngineState::new();
        let mut working_set = StateWorkingSet::new(&engine_state);

        let block = parse(&mut working_set, None, phrase, true);

        assert!(working_set.parse_errors.is_empty());
        assert_eq!(block.len(), 1, "{tag}: block len 1");

        let pipeline = &block.pipelines[0];
        assert_eq!(pipeline.len(), 1, "{tag}: expression length");
        let element = &pipeline.elements[0];
        assert!(element.redirection.is_none());
        if let Expr::Range(range) = &element.expr.expr {
            if let Range {
                from: None,
                next: None,
                to: Some(_),
                operator:
                    RangeOperator {
                        inclusion: the_inclusion,
                        ..
                    },
            } = range.as_ref()
            {
                assert_eq!(
                    *the_inclusion, inclusion,
                    "{tag}: wrong RangeInclusion {the_inclusion:?}"
                );
            } else {
                panic!("{tag}: expression mismatch.")
            }
        } else {
            panic!("{tag}: expression mismatch.")
        };
    }

    #[rstest]
    #[case(b"2.0..4.0..10.0", RangeInclusion::Inclusive, "float inclusive")]
    #[case(b"2.0..4.0..=10.0", RangeInclusion::Inclusive, "float =inclusive")]
    #[case(b"2.0..4.0..<10.0", RangeInclusion::RightExclusive, "float exclusive")]

    fn parse_float_range(
        #[case] phrase: &[u8],
        #[case] inclusion: RangeInclusion,
        #[case] tag: &str,
    ) {
        let engine_state = EngineState::new();
        let mut working_set = StateWorkingSet::new(&engine_state);

        let block = parse(&mut working_set, None, phrase, true);

        assert!(working_set.parse_errors.is_empty());
        assert_eq!(block.len(), 1, "{tag}: block length 1");

        let pipeline = &block.pipelines[0];
        assert_eq!(pipeline.len(), 1, "{tag}: expression length");
        let element = &pipeline.elements[0];
        assert!(element.redirection.is_none());
        if let Expr::Range(range) = &element.expr.expr {
            if let Range {
                from: Some(_),
                next: Some(_),
                to: Some(_),
                operator:
                    RangeOperator {
                        inclusion: the_inclusion,
                        ..
                    },
            } = range.as_ref()
            {
                assert_eq!(
                    *the_inclusion, inclusion,
                    "{tag}: wrong RangeInclusion {the_inclusion:?}"
                );
            } else {
                panic!("{tag}: expression mismatch.")
            }
        } else {
            panic!("{tag}: expression mismatch.")
        };
    }

    #[test]
    fn bad_parse_does_crash() {
        let engine_state = EngineState::new();
        let mut working_set = StateWorkingSet::new(&engine_state);

        let _ = parse(&mut working_set, None, b"(0)..\"a\"", true);

        assert!(!working_set.parse_errors.is_empty());
    }

    #[test]
    fn vars_not_read_as_units() {
        let engine_state = EngineState::new();
        let mut working_set = StateWorkingSet::new(&engine_state);

        let _ = parse(&mut working_set, None, b"0..<$day", true);

        assert!(working_set.parse_errors.is_empty());
    }

    #[rstest]
    #[case("(to-custom)..")]
    #[case("..(to-custom)")]
    #[case("(to-custom)..0")]
    #[case("..(to-custom)..0")]
    #[case("(to-custom)..0..")]
    #[case("(to-custom)..0..1")]
    #[case("0..(to-custom)")]
    #[case("0..(to-custom)..")]
    #[case("0..(to-custom)..1")]
    #[case("..1..(to-custom)")]
    #[case("0..1..(to-custom)")]
    fn type_mismatch_errors(#[case] code: &str) {
        let engine_state = EngineState::new();
        let mut working_set = StateWorkingSet::new(&engine_state);
        working_set.add_decl(Box::new(ToCustom));

        let _ = parse(&mut working_set, None, code.as_bytes(), true);

        assert!(matches!(
            &working_set.parse_errors[..],
            [ParseError::OperatorUnsupportedType { .. }]
        ),);
    }

    #[test]
    fn dont_mess_with_external_calls() {
        let engine_state = EngineState::new();
        let mut working_set = StateWorkingSet::new(&engine_state);
        working_set.add_decl(Box::new(ToCustom));

        let result = parse(&mut working_set, None, b"../foo", true);

        assert!(
            working_set.parse_errors.is_empty(),
            "Errors: {:?}",
            working_set.parse_errors
        );
        let expr = &result.pipelines[0].elements[0].expr.expr;
        assert!(
            matches!(expr, Expr::ExternalCall(..)),
            "Should've been parsed as a call"
        );
    }
}

#[cfg(test)]
mod mock {
    use super::*;
    use nu_engine::CallExt;
    use nu_protocol::{
        Category, IntoPipelineData, PipelineData, ShellError, Type, Value, engine::Call,
    };

    #[derive(Clone)]
    pub struct Const;

    impl Command for Const {
        fn name(&self) -> &str {
            "const"
        }

        fn description(&self) -> &str {
            "Create a parse-time constant."
        }

        fn signature(&self) -> nu_protocol::Signature {
            Signature::build("const")
                .input_output_types(vec![(Type::Nothing, Type::Nothing)])
                .allow_variants_without_examples(true)
                .required("const_name", SyntaxShape::VarWithOptType, "Constant name.")
                .required(
                    "initial_value",
                    SyntaxShape::Keyword(b"=".to_vec(), Box::new(SyntaxShape::MathExpression)),
                    "Equals sign followed by constant value.",
                )
                .category(Category::Core)
        }

        fn run(
            &self,
            _engine_state: &EngineState,
            _stack: &mut Stack,
            _call: &Call,
            _input: PipelineData,
        ) -> Result<PipelineData, ShellError> {
            todo!()
        }

        fn run_const(
            &self,
            _working_set: &StateWorkingSet,
            _call: &Call,
            _input: PipelineData,
        ) -> Result<PipelineData, ShellError> {
            Ok(PipelineData::empty())
        }
    }

    #[derive(Clone)]
    pub struct Let;

    impl Command for Let {
        fn name(&self) -> &str {
            "let"
        }

        fn description(&self) -> &str {
            "Create a variable and give it a value."
        }

        fn signature(&self) -> nu_protocol::Signature {
            Signature::build("let")
                .required("var_name", SyntaxShape::VarWithOptType, "variable name")
                .required(
                    "initial_value",
                    SyntaxShape::Keyword(b"=".to_vec(), Box::new(SyntaxShape::MathExpression)),
                    "equals sign followed by value",
                )
        }

        fn run(
            &self,
            _engine_state: &EngineState,
            _stack: &mut Stack,
            _call: &Call,
            _input: PipelineData,
        ) -> Result<PipelineData, ShellError> {
            todo!()
        }
    }

    #[derive(Clone)]
    pub struct Mut;

    impl Command for Mut {
        fn name(&self) -> &str {
            "mut"
        }

        fn description(&self) -> &str {
            "Mock mut command."
        }

        fn signature(&self) -> nu_protocol::Signature {
            Signature::build("mut")
                .required("var_name", SyntaxShape::VarWithOptType, "variable name")
                .required(
                    "initial_value",
                    SyntaxShape::Keyword(b"=".to_vec(), Box::new(SyntaxShape::MathExpression)),
                    "equals sign followed by value",
                )
        }

        fn run(
            &self,
            _engine_state: &EngineState,
            _stack: &mut Stack,
            _call: &Call,
            _input: PipelineData,
        ) -> Result<PipelineData, ShellError> {
            todo!()
        }
    }
    #[derive(Clone)]
    pub struct LsTest;

    impl Command for LsTest {
        fn name(&self) -> &str {
            "ls"
        }

        fn description(&self) -> &str {
            "Mock ls command."
        }

        fn signature(&self) -> nu_protocol::Signature {
            Signature::build(self.name()).category(Category::Default)
        }

        fn run(
            &self,
            _engine_state: &EngineState,
            _stack: &mut Stack,
            _call: &Call,
            _input: PipelineData,
        ) -> Result<PipelineData, ShellError> {
            todo!()
        }
    }

    #[derive(Clone)]
    pub struct Def;

    impl Command for Def {
        fn name(&self) -> &str {
            "def"
        }

        fn description(&self) -> &str {
            "Mock def command."
        }

        fn signature(&self) -> nu_protocol::Signature {
            Signature::build("def")
                .input_output_types(vec![(Type::Nothing, Type::Nothing)])
                .required("def_name", SyntaxShape::String, "definition name")
                .required("params", SyntaxShape::Signature, "parameters")
                .required("body", SyntaxShape::Closure(None), "body of the definition")
                .category(Category::Core)
        }

        fn run(
            &self,
            _engine_state: &EngineState,
            _stack: &mut Stack,
            _call: &Call,
            _input: PipelineData,
        ) -> Result<PipelineData, ShellError> {
            todo!()
        }
    }

    #[derive(Clone)]
    pub struct Alias;

    impl Command for Alias {
        fn name(&self) -> &str {
            "alias"
        }

        fn description(&self) -> &str {
            "Mock alias command."
        }

        fn signature(&self) -> nu_protocol::Signature {
            Signature::build("alias")
                .input_output_types(vec![(Type::Nothing, Type::Nothing)])
                .required("name", SyntaxShape::String, "Name of the alias.")
                .required(
                    "initial_value",
                    SyntaxShape::Keyword(b"=".to_vec(), Box::new(SyntaxShape::Expression)),
                    "Equals sign followed by value.",
                )
                .category(Category::Core)
        }

        fn run(
            &self,
            _engine_state: &EngineState,
            _stack: &mut Stack,
            _call: &Call,
            _input: PipelineData,
        ) -> Result<PipelineData, ShellError> {
            todo!()
        }
    }

    #[derive(Clone)]
    pub struct AttrEcho;

    impl Command for AttrEcho {
        fn name(&self) -> &str {
            "attr echo"
        }

        fn signature(&self) -> Signature {
            Signature::build("attr echo").required(
                "value",
                SyntaxShape::Any,
                "Value to store as an attribute",
            )
        }

        fn description(&self) -> &str {
            "Add an arbitrary value as an attribute to a command"
        }

        fn run(
            &self,
            engine_state: &EngineState,
            stack: &mut Stack,
            call: &Call,
            _input: PipelineData,
        ) -> Result<PipelineData, ShellError> {
            let value: Value = call.req(engine_state, stack, 0)?;
            Ok(value.into_pipeline_data())
        }

        fn is_const(&self) -> bool {
            true
        }

        fn run_const(
            &self,
            working_set: &StateWorkingSet,
            call: &Call,
            _input: PipelineData,
        ) -> Result<PipelineData, ShellError> {
            let value: Value = call.req_const(working_set, 0)?;
            Ok(value.into_pipeline_data())
        }
    }

    #[derive(Clone)]
    pub struct GroupBy;

    impl Command for GroupBy {
        fn name(&self) -> &str {
            "group-by"
        }

        fn description(&self) -> &str {
            "Mock group-by command."
        }

        fn signature(&self) -> nu_protocol::Signature {
            Signature::build(self.name())
                .required("column", SyntaxShape::String, "column name")
                .category(Category::Default)
        }

        fn run(
            &self,
            _engine_state: &EngineState,
            _stack: &mut Stack,
            _call: &Call,
            _input: PipelineData,
        ) -> Result<PipelineData, ShellError> {
            todo!()
        }
    }

    #[derive(Clone)]
    pub struct ToCustom;

    impl Command for ToCustom {
        fn name(&self) -> &str {
            "to-custom"
        }

        fn description(&self) -> &str {
            "Mock converter command."
        }

        fn signature(&self) -> nu_protocol::Signature {
            Signature::build(self.name())
                .input_output_type(Type::Any, Type::Custom("custom".into()))
                .category(Category::Custom("custom".into()))
        }

        fn run(
            &self,
            _engine_state: &EngineState,
            _stack: &mut Stack,
            _call: &Call,
            _input: PipelineData,
        ) -> Result<PipelineData, ShellError> {
            todo!()
        }
    }

    #[derive(Clone)]
    pub struct GroupByCustom;

    impl Command for GroupByCustom {
        fn name(&self) -> &str {
            "group-by"
        }

        fn description(&self) -> &str {
            "Mock custom group-by command."
        }

        fn signature(&self) -> nu_protocol::Signature {
            Signature::build(self.name())
                .required("column", SyntaxShape::String, "column name")
                .required("other", SyntaxShape::String, "other value")
                .input_output_type(Type::Custom("custom".into()), Type::Custom("custom".into()))
                .category(Category::Custom("custom".into()))
        }

        fn run(
            &self,
            _engine_state: &EngineState,
            _stack: &mut Stack,
            _call: &Call,
            _input: PipelineData,
        ) -> Result<PipelineData, ShellError> {
            todo!()
        }
    }

    #[derive(Clone)]
    pub struct AggCustom;

    impl Command for AggCustom {
        fn name(&self) -> &str {
            "agg"
        }

        fn description(&self) -> &str {
            "Mock custom agg command."
        }

        fn signature(&self) -> nu_protocol::Signature {
            Signature::build(self.name())
                .required("operation", SyntaxShape::String, "operation")
                .input_output_type(Type::Custom("custom".into()), Type::Custom("custom".into()))
                .category(Category::Custom("custom".into()))
        }

        fn run(
            &self,
            _engine_state: &EngineState,
            _stack: &mut Stack,
            _call: &Call,
            _input: PipelineData,
        ) -> Result<PipelineData, ShellError> {
            todo!()
        }
    }

    #[derive(Clone)]
    pub struct AggMin;

    impl Command for AggMin {
        fn name(&self) -> &str {
            "min"
        }

        fn description(&self) -> &str {
            "Mock custom min command."
        }

        fn signature(&self) -> nu_protocol::Signature {
            Signature::build(self.name()).category(Category::Custom("custom".into()))
        }

        fn run(
            &self,
            _engine_state: &EngineState,
            _stack: &mut Stack,
            _call: &Call,
            _input: PipelineData,
        ) -> Result<PipelineData, ShellError> {
            todo!()
        }
    }

    #[derive(Clone)]
    pub struct WithColumn;

    impl Command for WithColumn {
        fn name(&self) -> &str {
            "with-column"
        }

        fn description(&self) -> &str {
            "Mock custom with-column command."
        }

        fn signature(&self) -> nu_protocol::Signature {
            Signature::build(self.name())
                .rest("operation", SyntaxShape::Any, "operation")
                .input_output_type(Type::Custom("custom".into()), Type::Custom("custom".into()))
                .category(Category::Custom("custom".into()))
        }

        fn run(
            &self,
            _engine_state: &EngineState,
            _stack: &mut Stack,
            _call: &Call,
            _input: PipelineData,
        ) -> Result<PipelineData, ShellError> {
            todo!()
        }
    }

    #[derive(Clone)]
    pub struct Collect;

    impl Command for Collect {
        fn name(&self) -> &str {
            "collect"
        }

        fn description(&self) -> &str {
            "Mock custom collect command."
        }

        fn signature(&self) -> nu_protocol::Signature {
            Signature::build(self.name())
                .input_output_type(Type::Custom("custom".into()), Type::Custom("custom".into()))
                .category(Category::Custom("custom".into()))
        }

        fn run(
            &self,
            _engine_state: &EngineState,
            _stack: &mut Stack,
            _call: &Call,
            _input: PipelineData,
        ) -> Result<PipelineData, ShellError> {
            todo!()
        }
    }

    #[derive(Clone)]
    pub struct IfMocked;

    impl Command for IfMocked {
        fn name(&self) -> &str {
            "if"
        }

        fn description(&self) -> &str {
            "Mock if command."
        }

        fn signature(&self) -> nu_protocol::Signature {
            Signature::build("if")
                .required("cond", SyntaxShape::MathExpression, "condition to check")
                .required(
                    "then_block",
                    SyntaxShape::Block,
                    "block to run if check succeeds",
                )
                .optional(
                    "else_expression",
                    SyntaxShape::Keyword(
                        b"else".to_vec(),
                        Box::new(SyntaxShape::OneOf(vec![
                            SyntaxShape::Block,
                            SyntaxShape::Expression,
                        ])),
                    ),
                    "expression or block to run if check fails",
                )
                .category(Category::Core)
        }

        fn run(
            &self,
            _engine_state: &EngineState,
            _stack: &mut Stack,
            _call: &Call,
            _input: PipelineData,
        ) -> Result<PipelineData, ShellError> {
            todo!()
        }

        fn is_const(&self) -> bool {
            true
        }

        fn run_const(
            &self,
            _working_set: &StateWorkingSet,
            _call: &Call,
            _input: PipelineData,
        ) -> Result<PipelineData, ShellError> {
            panic!("Should not be called!")
        }
    }
}

#[cfg(test)]
mod input_types {
    use super::*;
    use mock::*;
    use nu_protocol::ast::Argument;

    fn add_declarations(engine_state: &mut EngineState) {
        let delta = {
            let mut working_set = StateWorkingSet::new(engine_state);
            working_set.add_decl(Box::new(Let));
            working_set.add_decl(Box::new(Def));
            working_set.add_decl(Box::new(AggCustom));
            working_set.add_decl(Box::new(GroupByCustom));
            working_set.add_decl(Box::new(GroupBy));
            working_set.add_decl(Box::new(LsTest));
            working_set.add_decl(Box::new(ToCustom));
            working_set.add_decl(Box::new(AggMin));
            working_set.add_decl(Box::new(Collect));
            working_set.add_decl(Box::new(WithColumn));
            working_set.add_decl(Box::new(IfMocked));
            working_set.add_decl(Box::new(Mut));

            working_set.render()
        };

        engine_state
            .merge_delta(delta)
            .expect("Error merging delta");
    }

    #[test]
    fn call_non_custom_types_test() {
        let mut engine_state = EngineState::new();
        add_declarations(&mut engine_state);

        let mut working_set = StateWorkingSet::new(&engine_state);
        let input = r#"ls | group-by name"#;

        let block = parse(&mut working_set, None, input.as_bytes(), true);

        assert!(working_set.parse_errors.is_empty());
        assert_eq!(block.len(), 1);

        let pipeline = &block.pipelines[0];
        assert_eq!(pipeline.len(), 2);
        assert!(pipeline.elements[0].redirection.is_none());
        assert!(pipeline.elements[1].redirection.is_none());

        match &pipeline.elements[0].expr.expr {
            Expr::Call(call) => {
                let expected_id = working_set.find_decl(b"ls").unwrap();
                assert_eq!(call.decl_id, expected_id)
            }
            _ => panic!("Expected expression Call not found"),
        }

        match &pipeline.elements[1].expr.expr {
            Expr::Call(call) => {
                let expected_id = working_set.find_decl(b"group-by").unwrap();
                assert_eq!(call.decl_id, expected_id)
            }
            _ => panic!("Expected expression Call not found"),
        }
    }

    #[test]
    fn nested_operations_test() {
        let mut engine_state = EngineState::new();
        add_declarations(&mut engine_state);

        let (block, delta) = {
            let mut working_set = StateWorkingSet::new(&engine_state);
            let input = r#"ls | to-custom | group-by name other | agg ("b" | min)"#;
            let block = parse(&mut working_set, None, input.as_bytes(), true);

            (block, working_set.render())
        };

        engine_state.merge_delta(delta).unwrap();

        let pipeline = &block.pipelines[0];
        assert!(pipeline.elements[3].redirection.is_none());
        match &pipeline.elements[3].expr.expr {
            Expr::Call(call) => {
                let arg = &call.arguments[0];
                match arg {
                    Argument::Positional(a) => match &a.expr {
                        Expr::FullCellPath(path) => match &path.head.expr {
                            Expr::Subexpression(id) => {
                                let block = engine_state.get_block(*id);

                                let pipeline = &block.pipelines[0];
                                assert_eq!(pipeline.len(), 2);
                                assert!(pipeline.elements[1].redirection.is_none());

                                match &pipeline.elements[1].expr.expr {
                                    Expr::Call(call) => {
                                        let working_set = StateWorkingSet::new(&engine_state);
                                        let expected_id = working_set.find_decl(b"min").unwrap();
                                        assert_eq!(call.decl_id, expected_id)
                                    }
                                    _ => panic!("Expected expression Call not found"),
                                }
                            }
                            _ => panic!("Expected Subexpression not found"),
                        },
                        _ => panic!("Expected FullCellPath not found"),
                    },
                    _ => panic!("Expected Argument Positional not found"),
                }
            }
            _ => panic!("Expected expression Call not found"),
        }
    }

    #[test]
    fn call_with_list_test() {
        let mut engine_state = EngineState::new();
        add_declarations(&mut engine_state);

        let mut working_set = StateWorkingSet::new(&engine_state);
        let input = r#"[[a b]; [1 2] [3 4]] | to-custom | with-column [ ("a" | min) ("b" | min) ] | collect"#;

        let block = parse(&mut working_set, None, input.as_bytes(), true);

        assert!(working_set.parse_errors.is_empty());
        assert_eq!(block.len(), 1);

        let pipeline = &block.pipelines[0];
        assert!(pipeline.elements[2].redirection.is_none());
        assert!(pipeline.elements[3].redirection.is_none());

        match &pipeline.elements[2].expr.expr {
            Expr::Call(call) => {
                let expected_id = working_set.find_decl(b"with-column").unwrap();
                assert_eq!(call.decl_id, expected_id)
            }
            _ => panic!("Expected expression Call not found"),
        }

        match &pipeline.elements[3].expr.expr {
            Expr::Call(call) => {
                let expected_id = working_set.find_decl(b"collect").unwrap();
                assert_eq!(call.decl_id, expected_id)
            }
            _ => panic!("Expected expression Call not found"),
        }
    }

    #[test]
    fn comments_within_blocks_test() {
        // https://github.com/nushell/nushell/issues/15305
        let mut engine_state = EngineState::new();
        add_declarations(&mut engine_state);

        let mut working_set = StateWorkingSet::new(&engine_state);
        let input = "def dummy []: int -> int { $in }";
        parse(&mut working_set, None, input.as_bytes(), true);

        for prefix in ["let ", "mut ", "mut foo = 1; $"] {
            let input = format!(
                r#"{prefix}foo = 1 |
                # comment
                dummy"#
            );
            let block = parse(&mut working_set, None, input.as_bytes(), true);
            let last_expr = &block.pipelines.last().unwrap().elements[0].expr.expr;
            let block_expr = match last_expr {
                Expr::Call(call) => {
                    assert_eq!(call.arguments.len(), 2);
                    call.arguments[1].expr().unwrap()
                }
                Expr::BinaryOp(_, _, rhs) => rhs.as_ref(),
                _ => panic!("Unexpected expression: {last_expr:?}"),
            };
            let block_id = match block_expr.expr {
                Expr::Block(block_id) | Expr::Subexpression(block_id) => block_id,
                _ => panic!("Unexpected expression: {block_expr:?}"),
            };
            let rhs_expr = working_set.get_block(block_id);
            assert_eq!(rhs_expr.pipelines.len(), 1);
            assert_eq!(rhs_expr.pipelines[0].elements.len(), 2);
            assert!(working_set.parse_errors.is_empty());
        }
    }

    #[test]
    fn operations_within_blocks_test() {
        let mut engine_state = EngineState::new();
        add_declarations(&mut engine_state);

        let mut working_set = StateWorkingSet::new(&engine_state);
        let inputs = vec![
            r#"let a = 'b'; ($a == 'b') or ($a == 'b')"#,
            r#"let a = 'b'; ($a == 'b') or ($a == 'b') and ($a == 'b')"#,
            r#"let a = 1; ($a == 1) or ($a == 2) and ($a == 3)"#,
            r#"let a = 'b'; if ($a == 'b') or ($a == 'b') { true } else { false }"#,
            r#"let a = 1; if ($a == 1) or ($a > 0) { true } else { false }"#,
        ];

        for input in inputs {
            let block = parse(&mut working_set, None, input.as_bytes(), true);

            assert!(working_set.parse_errors.is_empty());
            assert_eq!(block.len(), 2, "testing: {input}");
        }
    }

    #[test]
    fn closure_in_block_position_errors_correctly() {
        let mut engine_state = EngineState::new();
        add_declarations(&mut engine_state);

        let mut working_set = StateWorkingSet::new(&engine_state);
        let inputs = [r#"if true { || print hi }"#, r#"if true { |x| $x }"#];

        for input in inputs {
            parse(&mut working_set, None, input.as_bytes(), true);
            assert!(
                matches!(
                    working_set.parse_errors.first(),
                    Some(ParseError::Mismatch(_, _, _))
                ),
                "testing: {input}"
            );
        }
    }

    #[test]
    fn else_errors_correctly() {
        let mut engine_state = EngineState::new();
        add_declarations(&mut engine_state);

        let mut working_set = StateWorkingSet::new(&engine_state);
        let block = parse(
            &mut working_set,
            None,
            b"if false { 'a' } else { $foo }",
            true,
        );

        assert!(matches!(
            working_set.parse_errors.first(),
            Some(ParseError::VariableNotFound(_, _))
        ));

        let element = &block
            .pipelines
            .first()
            .unwrap()
            .elements
            .first()
            .unwrap()
            .expr;
        let Expr::Call(call) = &element.expr else {
            panic!("Expected Expr::Call, but found {:?}", element.expr);
        };
        let Expr::Keyword(else_kwd) = &call
            .arguments
            .get(2)
            .expect("This call of `if` should have 3 arguments")
            .expr()
            .unwrap()
            .expr
        else {
            panic!("Expected Expr::Keyword");
        };
        assert!(!matches!(else_kwd.expr.expr, Expr::Garbage))
    }

    #[test]
    fn else_if_errors_correctly() {
        let mut engine_state = EngineState::new();
        add_declarations(&mut engine_state);

        let mut working_set = StateWorkingSet::new(&engine_state);
        parse(
            &mut working_set,
            None,
            b"if false { 'a' } else $foo { 'b' }",
            true,
        );

        assert!(matches!(
            working_set.parse_errors.first(),
            Some(ParseError::VariableNotFound(_, _))
        ));
    }

    #[rstest]
    #[case::input_output(b"def q []: int -> int {1}", false)]
    #[case::input_output(b"def q [x: bool]: int -> int {2}", false)]
    #[case::input_output(b"def q []: string -> string {'qwe'}", false)]
    #[case::input_output(b"def q []: nothing -> nothing {null}", false)]
    #[case::input_output(b"def q []: list<string> -> list<string> {[]}", false)]
    #[case::input_output(
        b"def q []: record<a: int b: int> -> record<c: int e: int> {{c: 1 e: 1}}",
        false
    )]
    #[case::input_output(
        b"def q []: table<a: int b: int> -> table<c: int e: int> {[{c: 1 e: 1}]}",
        false
    )]
    #[case::input_output(
        b"def q []: nothing -> record<c: record<a: int b: int> e: int> {{c: {a: 1 b: 2} e: 1}}",
        false
    )]
    #[case::input_output(b"def q []: nothing -> list<string {[]}", true)]
    #[case::input_output(b"def q []: nothing -> record<c: int e: int {{c: 1 e: 1}}", true)]
    #[case::input_output(b"def q []: record<c: int e: int -> record<a: int> {{a: 1}}", true)]
    #[case::input_output(b"def q []: nothing -> record<a: record<a: int> {{a: {a: 1}}}", true)]
    #[case::input_output(b"def q []: int []}", true)]
    #[case::input_output(b"def q []: bool {[]", true)]
    // Type signature variants with whitespace between inputs and `:`
    #[case::input_output(b"def q [] : int -> int {1}", false)]
    #[case::input_output(b"def q [x: bool] : int -> int {2}", false)]
    #[case::input_output(b"def q []\t   : string -> string {'qwe'}", false)]
    #[case::input_output(b"def q []  \t : nothing -> nothing {null}", false)]
    #[case::input_output(b"def q [] \t: list<string> -> list<string> {[]}", false)]
    #[case::input_output(
        b"def q []\t: record<a: int b: int> -> record<c: int e: int> {{c: 1 e: 1}}",
        false
    )]
    #[case::input_output(
        b"def q [] : table<a: int b: int> -> table<c: int e: int> {[{c: 1 e: 1}]}",
        false
    )]
    #[case::input_output(
        b"def q [] : nothing -> record<c: record<a: int b: int> e: int> {{c: {a: 1 b: 2} e: 1}}",
        false
    )]
    #[case::input_output(b"def q [] : nothing -> list<string {[]}", true)]
    #[case::input_output(b"def q [] : nothing -> record<c: int e: int {{c: 1 e: 1}}", true)]
    #[case::input_output(b"def q [] : record<c: int e: int -> record<a: int> {{a: 1}}", true)]
    #[case::input_output(b"def q [] : nothing -> record<a: record<a: int> {{a: {a: 1}}}", true)]
    #[case::input_output(b"def q [] : int []}", true)]
    #[case::input_output(b"def q [] : bool {[]", true)]
    // No input-output type signature
    #[case::input_output(b"def qq [] {[]}", false)]
    #[case::input_output(b"def q [] []}", true)]
    #[case::input_output(b"def q [] {", true)]
    #[case::input_output(b"def q []: []}", true)]
    #[case::input_output(b"def q [] int {}", true)]
    #[case::input_output(b"def q [x: string, y: int] {{c: 1 e: 1}}", false)]
    #[case::input_output(b"def q [x: string, y: int]: {}", true)]
    #[case::input_output(b"def q [x: string, y: int] {a: {a: 1}}", true)]
    #[case::input_output(b"def foo {3}", true)]
    #[case::vardecl(b"let a: int = 1", false)]
    #[case::vardecl(b"let a: string = 'qwe'", false)]
    #[case::vardecl(b"let a: nothing = null", false)]
    #[case::vardecl(b"let a: list<string> = []", false)]
    #[case::vardecl(b"let a: record<a: int b: int> = {a: 1 b: 1}", false)]
    #[case::vardecl(
        b"let a: record<c: record<a: int b: int> e: int> = {c: {a: 1 b: 2} e: 1}",
        false
    )]
    #[case::vardecl(b"let a: table<a: int b: int> = [[a b]; [1 1]]", false)]
    #[case::vardecl(b"let a: list<string asd> = []", true)]
    #[case::vardecl(b"let a: record<a: int b: record<a: int> = {a: 1 b: {a: 1}}", true)]
    fn test_type_annotations(#[case] phrase: &[u8], #[case] expect_errors: bool) {
        let mut engine_state = EngineState::new();
        add_declarations(&mut engine_state);
        let mut working_set = StateWorkingSet::new(&engine_state);
        // this should not panic
        let _block = parse(&mut working_set, None, phrase, false);
        // check that no parse errors happened
        assert_eq!(
            !working_set.parse_errors.is_empty(),
            expect_errors,
            "Got errors {:?}",
            working_set.parse_errors
        )
    }
}

#[cfg(test)]
mod operator {
    use super::*;

    #[rstest]
    #[case(br#""abc" < "bca""#, "string < string")]
    #[case(br#""abc" <= "bca""#, "string <= string")]
    #[case(br#""abc" > "bca""#, "string > string")]
    #[case(br#""abc" >= "bca""#, "string >= string")]
    fn parse_comparison_operators_with_string_and_string(
        #[case] expr: &[u8],
        #[case] test_tag: &str,
    ) {
        let engine_state = EngineState::new();
        let mut working_set = StateWorkingSet::new(&engine_state);
        parse(&mut working_set, None, expr, false);
        assert_eq!(
            working_set.parse_errors.len(),
            0,
            "{test_tag}: expected to be parsed successfully, but failed."
        );
    }
}

mod record {
    use super::*;

    use nu_protocol::ast::RecordItem;

    #[rstest]
    #[case(b"{ :: x }", "Invalid literal")] // Key is bare colon
    #[case(b"{ a: x:y }", "Invalid literal")] // Value is bare word with colon
    #[case(b"{ a: x('y'):z }", "Invalid literal")] // Value is bare string interpolation with colon
    #[case(b"{ ;: x }", "Parse mismatch during operation.")] // Key is a non-item token
    #[case(b"{ a: || }", "Parse mismatch during operation.")] // Value is a non-item token
    fn refuse_confusing_record(#[case] expr: &[u8], #[case] error: &str) {
        dbg!(String::from_utf8_lossy(expr));
        let engine_state = EngineState::new();
        let mut working_set = StateWorkingSet::new(&engine_state);
        parse(&mut working_set, None, expr, false);
        assert_eq!(
            working_set.parse_errors.first().map(|e| e.to_string()),
            Some(error.to_string())
        );
    }

    #[rstest]
    #[case(b"{ a: 2024-07-23T22:54:54.532100627+02:00 b:xy }")]
    fn parse_datetime_in_record(#[case] expr: &[u8]) {
        dbg!(String::from_utf8_lossy(expr));
        let engine_state = EngineState::new();
        let mut working_set = StateWorkingSet::new(&engine_state);
        let block = parse(&mut working_set, None, expr, false);
        assert!(working_set.parse_errors.is_empty());
        let pipeline_el_expr = &block
            .pipelines
            .first()
            .unwrap()
            .elements
            .first()
            .unwrap()
            .expr
            .expr;
        dbg!(pipeline_el_expr);
        match pipeline_el_expr {
            Expr::FullCellPath(v) => match &v.head.expr {
                Expr::Record(fields) => assert!(matches!(
                    fields[0],
                    RecordItem::Pair(_, Expression { ty: Type::Date, .. })
                )),
                _ => panic!("Expected record head"),
            },
            _ => panic!("Expected full cell path"),
        }
    }

    /// Regression test for https://github.com/nushell/nushell/issues/15243
    #[test]
    fn record_terminate_loop() {
        let engine_state = EngineState::new();
        let mut working_set = StateWorkingSet::new(&engine_state);
        parse(&mut working_set, None, b"{a:b}/", false);
        assert_eq!(
            working_set.parse_errors.first().map(|e| e.to_string()),
            Some("Invalid characters after closing delimiter".to_string())
        );
    }
}
