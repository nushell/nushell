use nu_parser::*;
use nu_protocol::{
    ast::{Argument, Call, Expr, PathMember, Range},
    engine::{Command, EngineState, Stack, StateWorkingSet},
    ParseError, PipelineData, ShellError, Signature, Span, SyntaxShape,
};
use rstest::rstest;

#[cfg(test)]
#[derive(Clone)]
pub struct Let;

#[cfg(test)]
impl Command for Let {
    fn name(&self) -> &str {
        "let"
    }

    fn usage(&self) -> &str {
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
            let act_err = format!("{:?}", parse_err);
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
            Expr::String("./a/b".into()),
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
        assert_eq!(call.decl_id, 0);
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
        assert_eq!(call.decl_id, 0);
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
#[case(b"let a = 1 err> /dev/null")]
#[case(b"let a = 1 out> /dev/null")]
#[case(b"mut a = 1 err> /dev/null")]
#[case(b"mut a = 1 out> /dev/null")]
#[case(b"let a = 1 out+err> /dev/null")]
#[case(b"mut a = 1 out+err> /dev/null")]
fn test_redirection_with_letmut(#[case] phase: &[u8]) {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);
    let _block = parse(&mut working_set, None, phase, true);
    assert!(matches!(
        working_set.parse_errors.first(),
        Some(ParseError::RedirectingBuiltinCommand(_, _, _))
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
}

#[cfg(test)]
mod input_types {
    use super::*;
    use nu_protocol::{
        ast::{Argument, Call},
        Category, PipelineData, ShellError, Type,
    };

    #[derive(Clone)]
    pub struct LsTest;

    impl Command for LsTest {
        fn name(&self) -> &str {
            "ls"
        }

        fn usage(&self) -> &str {
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

        fn usage(&self) -> &str {
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
    pub struct GroupBy;

    impl Command for GroupBy {
        fn name(&self) -> &str {
            "group-by"
        }

        fn usage(&self) -> &str {
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

        fn usage(&self) -> &str {
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

        fn usage(&self) -> &str {
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

        fn usage(&self) -> &str {
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

        fn usage(&self) -> &str {
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

        fn usage(&self) -> &str {
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

        fn usage(&self) -> &str {
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

        fn usage(&self) -> &str {
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
    }

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
    fn else_errors_correctly() {
        let mut engine_state = EngineState::new();
        add_declarations(&mut engine_state);

        let mut working_set = StateWorkingSet::new(&engine_state);
        parse(
            &mut working_set,
            None,
            b"if false { 'a' } else { $foo }",
            true,
        );

        assert!(matches!(
            working_set.parse_errors.first(),
            Some(ParseError::VariableNotFound(_, _))
        ));
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
