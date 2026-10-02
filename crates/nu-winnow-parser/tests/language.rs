//! Structural checks mirroring Nushell's own lexer and parser tests.
//!
//! The inputs come from `crates/nu-parser/tests/test_lex.rs`,
//! `test_parser.rs`, `test_parser_unicode_escapes.rs` and the language tests
//! in `tests/repl/` of the Nushell repository; each table asserts the value
//! or shape nu-parser produces for the same text.
//!
//! `tests/fixtures/` covers accept/reject verdicts and pins whole trees; this
//! file pins individual values, spans and token streams.
#![allow(clippy::unwrap_used, clippy::format_push_string, reason = "tests may panic on failure")]

use std::path::PathBuf;

use nu_winnow_parser::ast::*;
use nu_winnow_parser::lex::{LexOptions, RedirectionOperator, TokenContents, lex};
use nu_winnow_parser::{ErrorKind, ParseConfig, Span, parse, parse_lenient};
use rstest::rstest;

// --- helpers ------------------------------------------------------------------

fn ok(src: &str) -> Ast<'_> {
    match parse(src) {
        Ok(ast) => ast,
        Err(e) => panic!("failed to parse {src:?}:\n{}", e.render(src, None)),
    }
}

fn err_text(src: &str) -> String {
    match parse(src) {
        Ok(ast) => panic!("expected {src:?} to fail, got:\n{}", nu_winnow_parser::pretty::dump(&ast)),
        Err(e) => e.diagnostics.iter().map(|d| format!("{}{}", d.kind, d.help.as_deref().unwrap_or(""))).collect(),
    }
}

/// The expression of the last pipeline element of a one-pipeline source.
fn last_expr<'a>(ast: &'a Ast<'a>) -> &'a Expression<'a> {
    let p = ast.block.pipelines.last().expect("a pipeline");
    &p.elements.last().expect("an element").expr
}

/// The right-most operand of a binary operation, or the expression itself
/// (the shape nu's `compare_rhs_binary_op` looks at).
fn rhs<'a>(e: &'a Expression<'a>) -> &'a Expression<'a> {
    match &e.expr {
        Expr::BinaryOp(b) => rhs(&b.rhs),
        _ => e,
    }
}

fn string_value(e: &Expression<'_>) -> (String, Quote) {
    match &e.expr {
        Expr::String(s) => (s.value.to_string(), s.quote),
        other => panic!("expected a string, got {other:?}"),
    }
}

/// A one-letter summary of each interpolation part: `T` text, `E` expression.
fn interp_shape(e: &Expression<'_>) -> (Quote, Vec<String>) {
    match &e.expr {
        Expr::StringInterpolation(i) => (
            i.quote,
            i.parts
                .iter()
                .map(|p| match p {
                    InterpolationPart::Text { value, .. } => format!("T:{value}"),
                    InterpolationPart::Expression(_) => "E".to_string(),
                })
                .collect(),
        ),
        other => panic!("expected an interpolation, got {other:?}"),
    }
}

/// Render an expression as a fully parenthesised string, to pin precedence.
fn sexpr(src: &str, e: &Expression<'_>) -> String {
    match &e.expr {
        Expr::BinaryOp(b) => format!("({} {} {})", sexpr(src, &b.lhs), b.op.item, sexpr(src, &b.rhs)),
        Expr::UnaryNot(n) => format!("(not {})", sexpr(src, &n.expr)),
        Expr::Subexpression(b) => {
            let inner = &b.pipelines[0].elements[0].expr;
            sexpr(src, inner)
        }
        _ => e.span.slice(src).to_string(),
    }
}

// --- numbers (test_parser.rs: multi_test_parse_int, parse_int*, repl/test_parser.rs) --------

#[rstest]
#[case("3", 3)]
#[case("420_69_2023", 420_692_023)]
#[case("1_0000_0000_0000 + 10", 10)]
#[case("0b_10100_11101_10010", 21426)]
#[case("0o2443_6442_7652_0044", 90_422_533_333_028)]
#[case("0x68__9d__6a", 6_856_042)]
#[case("0 + 0b0", 0)]
#[case("0 + 0o1", 1)]
#[case("0 + 0x2", 2)]
#[case("0 + 0x00e0", 0xe0)]
#[case("0 + 42", 42)]
#[case("0 + -42", -42)]
#[case("0 + +42", 42)]
#[case("0xffffffffffffffff", -1)]
#[case("0x42b", 1067)]
fn ints(#[case] src: &str, #[case] expected: i64) {
    let ast = ok(src);
    match &rhs(last_expr(&ast)).expr {
        Expr::Int(i) => assert_eq!(*i, expected, "{src}"),
        other => panic!("{src}: expected Int, got {other:?}"),
    }
}

#[rstest]
#[case("0 + 0b2", "radix 2")]
#[case("0 + 0o8", "radix 8")]
#[case("0 + 0o", "radix 8")]
#[case("0 + 0x0aq", "radix 16")]
#[case("echo 0b2", "radix 2")]
fn radix_prefixed_words_must_be_ints(#[case] src: &str, #[case] message: &str) {
    assert!(err_text(src).contains(message), "{src}: {}", err_text(src));
}

#[rstest]
#[case("43.5", 43.5)]
#[case("-41.7", -41.7)]
#[case("3e10", 3.0e10)]
#[case("0 + 43.5", 43.5)]
#[case("3.1415_9265_3589_793 * 2", 2.0)]
#[case(".5", 0.5)]
fn floats(#[case] src: &str, #[case] expected: f64) {
    let ast = ok(src);
    match &rhs(last_expr(&ast)).expr {
        Expr::Float(f) => assert_eq!(*f, expected, "{src}"),
        Expr::Int(i) => assert_eq!(*i as f64, expected, "{src}"),
        other => panic!("{src}: expected Float, got {other:?}"),
    }
}

/// nu parses these as strings (or an external head); they are never numbers.
#[rstest]
#[case("-x", "-x")]
#[case("--exact", "--exact")]
#[case("'1.0.1'", "1.0.1")]
#[case("echo 1.0.1", "1.0.1")]
#[case("echo 1_000_d_ay", "1_000_d_ay")]
fn number_like_strings(#[case] src: &str, #[case] expected: &str) {
    let ast = ok(src);
    let e = last_expr(&ast);
    let value = match &e.expr {
        Expr::Call(c) => string_value(c.positional_iter().next().expect("an argument")).0,
        _ => string_value(e).0,
    };
    assert_eq!(value, expected);
}

/// `./a/b` is no known command, so it is an external command, as in nu.
#[test]
fn relative_path_is_an_external_command() {
    let ast = ok("./a/b");
    match &last_expr(&ast).expr {
        Expr::ExternalCall(c) => {
            assert_eq!(c.caret, None);
            assert_eq!(string_value(&c.head).0, "./a/b");
        }
        other => panic!("{other:?}"),
    }
}

#[rstest]
#[case("95307.27MiB", FilesizeUnit::MiB, 99_936_915_947)]
#[case("420_MB", FilesizeUnit::MB, 420_000_000)]
#[case("1_000_000B", FilesizeUnit::B, 1_000_000)]
#[case("1kb", FilesizeUnit::KB, 1_000)]
#[case("1KiB", FilesizeUnit::KiB, 1_024)]
fn filesizes(#[case] src: &str, #[case] unit: FilesizeUnit, #[case] bytes: i64) {
    let ast = ok(src);
    match &last_expr(&ast).expr {
        Expr::Filesize(f) => {
            assert_eq!(f.unit, unit);
            assert_eq!(f.to_bytes(), Some(bytes));
        }
        other => panic!("{src}: {other:?}"),
    }
}

#[rstest]
#[case(".6min", DurationUnit::Minute, 36_000_000_000)]
#[case("420_min", DurationUnit::Minute, 7 * 3_600_000_000_000)]
#[case("1_000_000sec", DurationUnit::Second, 1_000_000_000_000_000)]
#[case("5ns", DurationUnit::Nanosecond, 5)]
#[case("1\u{00B5}s", DurationUnit::Microsecond, 1_000)]
fn durations(#[case] src: &str, #[case] unit: DurationUnit, #[case] nanoseconds: i64) {
    let ast = ok(src);
    match &last_expr(&ast).expr {
        Expr::Duration(d) => {
            assert_eq!(d.unit, unit);
            assert_eq!(d.to_nanoseconds(), Some(nanoseconds));
        }
        other => panic!("{src}: {other:?}"),
    }
}

#[rstest]
#[case("echo 42m_b", "filesize")]
#[case("sleep 4-ms", "duration")]
#[case("echo 1..2sec", "duration")]
fn unit_suffix_with_a_bad_number_is_an_error(#[case] src: &str, #[case] kind: &str) {
    let text = err_text(src);
    assert!(text.contains(&format!("invalid {kind} literal")), "{src}: {text}");
}

#[rstest]
#[case("0x[13]", &[0x13])]
#[case("0x[3]", &[0x03])]
#[case("0b[1010 1000]", &[0b1010_1000])]
#[case("0b[10]", &[0b10])]
#[case("0o[250]", &[0o250])]
#[case("0o[2]", &[0o2])]
#[case("0x[]", &[])]
fn binary_literals(#[case] src: &str, #[case] bytes: &[u8]) {
    let ast = ok(src);
    match &last_expr(&ast).expr {
        Expr::Binary(b) => assert_eq!(b.bytes, bytes),
        other => panic!("{src}: {other:?}"),
    }
}

#[rstest]
#[case("0o[90]")]
#[case("0x[zz]")]
#[case("0o[777]")]
fn invalid_binary_literals(#[case] src: &str) {
    let text = err_text(src);
    assert!(text.contains("invalid binary literal"), "{src}: {text}");
}

// --- strings (test_parser_unicode_escapes.rs, repl/test_parser.rs) --------------------------

#[rstest]
#[case(r#""hello \u{6e}\u{000075}\u{073}hell""#, "hello nushell")]
#[case(r#""\u{39}8\u{10ffff}""#, "98\u{10ffff}")]
#[case(r#""abc\u{41}""#, "abcA")]
#[case(r#""\u{41}abc""#, "Aabc")]
#[case(r#""\u{a}""#, "\n")]
#[case(r#""\u{015B}\u{1f10b}""#, "ś🄋")]
#[case(r#""hello nushell""#, "hello nushell")]
#[case(r#""\"=foo""#, "\"=foo")]
#[case("r##' one # two '##", " one # two ")]
#[case(r#"'a\nb'"#, r"a\nb")]
fn string_values(#[case] src: &str, #[case] expected: &str) {
    let ast = ok(src);
    assert_eq!(string_value(last_expr(&ast)).0, expected);
}

#[rstest]
#[case(r#""hello \u{6e""#, "missing closing '}'")]
#[case(r#""\u{39}8\u{000000000000000000000000000000000000000000000037}""#, "must be 1-6 hex digits")]
#[case(r#""\u{110000}""#, "max codepoint 0x10FFFF")]
#[case(r#""\u0041""#, "missing closing '}'")]
#[case(r#""\x4""#, "expected 2 hex digits")]
#[case(r#""\xzz""#, "expected 2 hex digits")]
#[case(r#""\q""#, "unrecognized escape")]
fn string_escape_errors(#[case] src: &str, #[case] message: &str) {
    let text = err_text(src);
    assert!(text.contains(message), "{src}: {text}");
}

#[test]
fn trailing_backslash_is_an_unclosed_quote() {
    assert!(matches!(parse("\"say \\").unwrap_err().primary().kind, ErrorKind::Unclosed { delimiter: "\"", .. }));
}

// --- interpolation (test_parser.rs: string::interpolation) ---------------------------------

#[rstest]
#[case(r#"$"hello (39 + 3)""#, Quote::Double, &["T:hello ", "E"])]
#[case(r#"$"hello \(39 + 3)""#, Quote::Double, &["T:hello (39 + 3)"])]
#[case(r#"$"hello \\(39 + 3)""#, Quote::Double, &["T:hello \\", "E"])]
#[case(r#"$"\(1 + 3)\(7 - 5)""#, Quote::Double, &["T:(1 + 3)(7 - 5)"])]
#[case(r#"$"2 + 2 is \(2 + 2)""#, Quote::Double, &["T:2 + 2 is (2 + 2)"])]
#[case(r#""" ++ foo(1 + 3)bar(7 - 5)"#, Quote::Bare, &["T:foo", "E", "T:bar", "E"])]
#[case(r#""" ++ (1 + 3)foo(7 - 5)bar"#, Quote::Bare, &["E", "T:foo", "E", "T:bar"])]
#[case("(100 + 20 + 3)/bar/(300 + 20 + 1)", Quote::Bare, &["E", "T:/bar/", "E"])]
#[case("echo ~/.foo/(1)", Quote::Bare, &["T:~/.foo/", "E"])]
#[case(r"echo ~\.foo(2)\(1)", Quote::Bare, &["T:~\\.foo", "E", "T:\\", "E"])]
#[case(r#"$"('(')(')')""#, Quote::Double, &["E", "E"])]
#[case(r#"$"('(')test(')')""#, Quote::Double, &["E", "T:test", "E"])]
#[case(r#"$'no (escapes) \(here)'"#, Quote::Single, &["T:no ", "E", "T: \\", "E"])]
#[case(r#"$"a()b""#, Quote::Double, &["T:a", "E", "T:b"])]
fn interpolation_parts(#[case] src: &str, #[case] quote: Quote, #[case] parts: &[&str]) {
    let ast = ok(src);
    let e = match &last_expr(&ast).expr {
        Expr::Call(c) => c.positional_iter().next().expect("an argument"),
        _ => rhs(last_expr(&ast)),
    };
    let (q, shape) = interp_shape(e);
    assert_eq!(q, quote, "{src}");
    assert_eq!(shape, parts, "{src}");
}

/// `~/.foo/(1)` at the head of a pipeline is an external call with an
/// interpolated name, as in nu.
#[test]
fn interpolated_bare_word_at_pipeline_head_is_an_external_call() {
    let ast = ok("~/.foo/(1)");
    match &last_expr(&ast).expr {
        Expr::ExternalCall(c) => assert!(matches!(c.head.expr, Expr::StringInterpolation(_)), "{:?}", c.head),
        other => panic!("{other:?}"),
    }
}

#[test]
fn interpolation_in_external_argument() {
    let ast = ok("^echo ($nu.home-path)/path");
    match &last_expr(&ast).expr {
        Expr::ExternalCall(c) => match &c.arguments[0] {
            ExternalArgument::Regular(e) => assert_eq!(interp_shape(e).1, ["E", "T:/path"]),
            other => panic!("{other:?}"),
        },
        other => panic!("{other:?}"),
    }
}

#[test]
fn unclosed_interpolation_subexpression_names_the_paren() {
    let e = parse("$\"foo (2 + 3\"").unwrap_err();
    match e.primary().kind {
        ErrorKind::Unclosed { delimiter: ")", open } => assert_eq!(open, Span::new(6, 7)),
        ref other => panic!("{other:?}"),
    }
}

// --- external calls (test_parser.rs: test_external_call_*) ----------------------------------

#[rstest]
#[case("^foo-external-call", "foo-external-call", Quote::Bare)]
#[case("^foo/external-call", "foo/external-call", Quote::Bare)]
#[case(r"^foo\external-call", r"foo\external-call", Quote::Bare)]
// Backtick words are unquoted in nu's external strings (`GlobPattern` with `is_quoted: false`).
#[case("^`foo external call`", "foo external call", Quote::Bare)]
#[case("^`foo/external call`", "foo/external call", Quote::Bare)]
#[case(r"^`foo\external call`", r"foo\external call", Quote::Bare)]
#[case("^'foo external call'", "foo external call", Quote::Single)]
#[case(r"^'foo\external call'", r"foo\external call", Quote::Single)]
#[case(r#"^"foo external call""#, "foo external call", Quote::Double)]
#[case(r#"^"foo\\external call""#, r"foo\external call", Quote::Double)]
#[case("^r#'foo-external-call'#", "foo-external-call", Quote::Raw(1))]
#[case(r##"^r#'foo\external-call'#"##, r"foo\external-call", Quote::Raw(1))]
fn external_call_heads(#[case] src: &str, #[case] expected: &str, #[case] quote: Quote) {
    let ast = ok(src);
    match &last_expr(&ast).expr {
        Expr::ExternalCall(c) => {
            assert_eq!(string_value(&c.head), (expected.to_string(), quote), "{src}");
            assert!(c.arguments.is_empty());
        }
        other => panic!("{src}: {other:?}"),
    }
}

#[rstest]
#[case("^~/.foo/(1)", 2, Quote::Bare)]
#[case(r#"^$"~/.foo/(1)""#, 2, Quote::Double)]
fn external_call_interpolated_heads(#[case] src: &str, #[case] parts: usize, #[case] quote: Quote) {
    let ast = ok(src);
    match &last_expr(&ast).expr {
        Expr::ExternalCall(c) => {
            let (q, shape) = interp_shape(&c.head);
            assert_eq!((q, shape.len()), (quote, parts), "{src}");
        }
        other => panic!("{src}: {other:?}"),
    }
}

#[rstest]
#[case("^foo foo-external-call", "foo-external-call", Quote::Bare)]
#[case("^foo foo/external-call", "foo/external-call", Quote::Bare)]
#[case(r"^foo foo\external-call", r"foo\external-call", Quote::Bare)]
#[case("^foo `foo external call`", "foo external call", Quote::Bare)]
#[case(r#"^foo --flag="value""#, "--flag=value", Quote::Bare)]
#[case("^foo --flag='value'", "--flag=value", Quote::Bare)]
#[case("^foo `hello world`", "hello world", Quote::Bare)]
#[case(r#"^foo `"hello world"`"#, "\"hello world\"", Quote::Bare)]
#[case("^foo `'hello world'`", "'hello world'", Quote::Bare)]
#[case("^foo r#'foo-external-call'#", "foo-external-call", Quote::Raw(1))]
#[case("^foo 'foo external call'", "foo external call", Quote::Single)]
#[case(r#"^foo "foo\\external call""#, r"foo\external call", Quote::Double)]
#[case("^foo '{a:1}'", "{a:1}", Quote::Single)]
#[case("^foo 'q($x)'", "q($x)", Quote::Single)]
fn external_call_string_args(#[case] src: &str, #[case] expected: &str, #[case] quote: Quote) {
    let ast = ok(src);
    match &last_expr(&ast).expr {
        Expr::ExternalCall(c) => {
            assert_eq!(string_value(&c.head).0, "foo");
            assert_eq!(c.arguments.len(), 1);
            match &c.arguments[0] {
                ExternalArgument::Regular(e) => assert_eq!(string_value(e), (expected.to_string(), quote), "{src}"),
                other => panic!("{src}: {other:?}"),
            }
        }
        other => panic!("{src}: {other:?}"),
    }
}

#[rstest]
#[case("^foo ~/.foo/(1)", 2, Quote::Bare)]
#[case(r#"^foo $"~/.foo/(1)""#, 2, Quote::Double)]
#[case("^foo --out=(pwd)/x", 3, Quote::Bare)]
fn external_call_interpolated_args(#[case] src: &str, #[case] parts: usize, #[case] quote: Quote) {
    let ast = ok(src);
    match &last_expr(&ast).expr {
        Expr::ExternalCall(c) => match &c.arguments[0] {
            ExternalArgument::Regular(e) => {
                let (q, shape) = interp_shape(e);
                assert_eq!((q, shape.len()), (quote, parts), "{src}");
            }
            other => panic!("{src}: {other:?}"),
        },
        other => panic!("{src}: {other:?}"),
    }
}

#[rstest]
#[case(r#"^foo {|my_var| $"($my_var)" }"#, "closure")]
#[case(r#"^foo { |my_var| $"($my_var)" }"#, "closure")]
#[case("^foo { 42 }", "closure")]
#[case("^foo {a: 1}", "record")]
#[case("^foo {}", "record")]
#[case("^foo {a:1,b:'c',c:'d'}", "record")]
#[case(r#"^foo {a:1,b:"c",c:"d"}"#, "record")]
#[case("^foo {a: 1, b: 'c', c: 'd'}", "record")]
#[case(r#"^foo {"key with spaces": "value"}"#, "record")]
#[case(r#"^foo {outer: {inner: "value"}}"#, "record")]
#[case(r#"^foo {items: [1 "two" true]}"#, "record")]
#[case("^foo [a b c]", "list")]
#[case("^foo $x", "var")]
#[case("^foo (pwd)", "subexpression")]
fn external_call_structured_args(#[case] src: &str, #[case] kind: &str) {
    let ast = ok(src);
    match &last_expr(&ast).expr {
        Expr::ExternalCall(c) => match &c.arguments[0] {
            ExternalArgument::Regular(e) => {
                let actual = match &e.expr {
                    Expr::Closure(_) => "closure",
                    Expr::Record(_) => "record",
                    Expr::List(_) => "list",
                    Expr::Var(_) => "var",
                    Expr::Subexpression(_) => "subexpression",
                    other => panic!("{src}: {other:?}"),
                };
                assert_eq!(actual, kind, "{src}");
            }
            other => panic!("{src}: {other:?}"),
        },
        other => panic!("{src}: {other:?}"),
    }
}

#[test]
fn external_call_spread_argument() {
    let ast = ok("^foo ...[a b c]");
    match &last_expr(&ast).expr {
        Expr::ExternalCall(c) => match &c.arguments[0] {
            ExternalArgument::Spread { expr, .. } => match &expr.expr {
                Expr::List(items) => assert_eq!(items.len(), 3),
                other => panic!("{other:?}"),
            },
            other => panic!("{other:?}"),
        },
        other => panic!("{other:?}"),
    }
}

/// `%name` forces the built-in; `%$var` and `%(expr)` dispatch dynamically.
#[rstest]
#[case("%ls", "ls", 0)]
#[case("% ls", "ls", 0)]
#[case("%echo ok", "echo", 1)]
#[case("%ls -a --long", "ls", 2)]
#[case("\" x \" | %str trim", "str trim", 0)]
fn percent_sigil_calls(#[case] src: &str, #[case] name: &str, #[case] args: usize) {
    let ast = ok(src);
    match &last_expr(&ast).expr {
        Expr::Call(c) => {
            assert_eq!(c.head.name, name, "{src}");
            assert_eq!(c.arguments.len(), args, "{src}");
            assert_eq!(c.sigil.map(|s| s.slice(src)), Some("%"), "{src}");
        }
        other => panic!("{src}: {other:?}"),
    }
}

#[rstest]
#[case("%$cmd hello", "var", 1)]
#[case("%('echo') hello", "subexpression", 1)]
#[case("% ('echo') hello", "subexpression", 1)]
#[case("%($cmd) ...$args", "subexpression", 1)]
#[case("%$x.0", "cell path", 0)]
fn percent_sigil_dynamic_calls(#[case] src: &str, #[case] head: &str, #[case] args: usize) {
    let ast = ok(src);
    match &last_expr(&ast).expr {
        Expr::DynamicCall(d) => {
            let actual = match &d.head.expr {
                Expr::Var(_) => "var",
                Expr::Subexpression(_) => "subexpression",
                Expr::FullCellPath(_) => "cell path",
                other => panic!("{src}: {other:?}"),
            };
            assert_eq!(actual, head, "{src}");
            assert_eq!(d.arguments.len(), args, "{src}");
            assert_eq!(d.sigil.slice(src), "%");
        }
        other => panic!("{src}: {other:?}"),
    }
}

#[rstest]
#[case("%nu --version")]
#[case("def foo [] { 'ok' }; %foo")]
#[case("%")]
#[case("%[1] x")]
#[case("%\"ls\"")]
fn percent_sigil_requires_a_builtin(#[case] src: &str) {
    assert!(err_text(src).contains("percent sigil requires a built-in command"), "{src}");
}

#[test]
fn percent_sigil_without_a_command_table_is_not_checked() {
    let config = ParseConfig::empty();
    let (_, diagnostics) = parse_lenient("%anything", &config);
    assert!(diagnostics.is_empty());
}

// --- cell paths and ranges (test_parser.rs: parse_cell_path*, mod range) -------------------

#[rstest]
#[case("$foo.bar.baz", &[("bar", false), ("baz", false)])]
#[case("$foo.bar?.baz", &[("bar", true), ("baz", false)])]
#[case("$foo.0.b?", &[("0", false), ("b", true)])]
fn cell_path_members(#[case] src: &str, #[case] expected: &[(&str, bool)]) {
    let ast = ok(src);
    match &last_expr(&ast).expr {
        Expr::FullCellPath(p) => {
            assert!(matches!(p.head.expr, Expr::Var(_)));
            let members: Vec<(String, bool)> = p
                .tail
                .iter()
                .map(|m| {
                    let name = match &m.kind {
                        PathMemberKind::String(s) => s.to_string(),
                        PathMemberKind::Int(i) => i.to_string(),
                    };
                    (name, m.optional)
                })
                .collect();
            let expected: Vec<(String, bool)> = expected.iter().map(|(n, o)| (n.to_string(), *o)).collect();
            assert_eq!(members, expected);
        }
        other => panic!("{src}: {other:?}"),
    }
}

#[rstest]
#[case("0..10", true, false, true, RangeInclusion::Inclusive)]
#[case("0..=10", true, false, true, RangeInclusion::Inclusive)]
#[case("0..<10", true, false, true, RangeInclusion::RightExclusive)]
#[case("10..0", true, false, true, RangeInclusion::Inclusive)]
#[case("10..=0", true, false, true, RangeInclusion::Inclusive)]
#[case("(3 - 3)..<(8 + 2)", true, false, true, RangeInclusion::RightExclusive)]
#[case("(3 - 3)..(8 + 2)", true, false, true, RangeInclusion::Inclusive)]
#[case("(3 - 3)..=(8 + 2)", true, false, true, RangeInclusion::Inclusive)]
#[case("-10..-3", true, false, true, RangeInclusion::Inclusive)]
#[case("-10..=-3", true, false, true, RangeInclusion::Inclusive)]
#[case("-10..<-3", true, false, true, RangeInclusion::RightExclusive)]
#[case("let a = 2; $a..10", true, false, true, RangeInclusion::Inclusive)]
#[case("let a = 2; $a..=10", true, false, true, RangeInclusion::Inclusive)]
#[case("let a = 2; $a..<($a + 10)", true, false, true, RangeInclusion::RightExclusive)]
#[case("0..", true, false, false, RangeInclusion::Inclusive)]
#[case("0..=", true, false, false, RangeInclusion::Inclusive)]
#[case("0..<", true, false, false, RangeInclusion::RightExclusive)]
#[case("..10", false, false, true, RangeInclusion::Inclusive)]
#[case("..=10", false, false, true, RangeInclusion::Inclusive)]
#[case("..<10", false, false, true, RangeInclusion::RightExclusive)]
#[case("2.0..4.0..10.0", true, true, true, RangeInclusion::Inclusive)]
#[case("2.0..4.0..=10.0", true, true, true, RangeInclusion::Inclusive)]
#[case("2.0..4.0..<10.0", true, true, true, RangeInclusion::RightExclusive)]
#[case("0..<$day", true, false, true, RangeInclusion::RightExclusive)]
#[case("0..(1..2 | first)", true, false, true, RangeInclusion::Inclusive)]
#[case("1..(5)..10", true, true, true, RangeInclusion::Inclusive)]
fn ranges(#[case] src: &str, #[case] from: bool, #[case] next: bool, #[case] to: bool, #[case] incl: RangeInclusion) {
    let ast = ok(src);
    match &last_expr(&ast).expr {
        Expr::Range(r) => {
            assert_eq!(
                (r.from.is_some(), r.next.is_some(), r.to.is_some(), r.operator.inclusion),
                (from, next, to, incl),
                "{src}"
            );
        }
        other => panic!("{src}: {other:?}"),
    }
}

#[rstest]
#[case("(0)..\"a\"")]
#[case("') ..")]
fn bad_ranges(#[case] src: &str) {
    assert!(parse(src).is_err(), "{src}");
}

#[test]
fn ranges_do_not_swallow_unit_like_variable_names() {
    for src in ["let runs = 10; 1..$runs", "let sizekb = 10; 1..$sizekb"] {
        let ast = ok(src);
        assert!(matches!(last_expr(&ast).expr, Expr::Range(_)), "{src}");
    }
}

// --- operators (repl/test_parser.rs, repl/test_math.rs) -------------------------------------

#[rstest]
#[case("1 + 2 * 3 - 4 / 2", "((1 + (2 * 3)) - (4 / 2))")]
#[case("2 ** 3 ** 2", "(2 ** (3 ** 2))")]
#[case("10 - 3 - 2", "((10 - 3) - 2)")]
#[case("true and false or true", "((true and false) or true)")]
#[case("true and true xor true and false", "((true and true) xor (true and false))")]
#[case("true or false xor true or false", "((true or (false xor true)) or false)")]
#[case("not $a and $b or $c > 1", "(((not $a) and $b) or ($c > 1))")]
#[case("not not true", "(not (not true))")]
#[case("1 bit-shl 2 bit-or 4 bit-and 7", "((1 bit-shl 2) bit-or (4 bit-and 7))")]
#[case("1 + 2 == 3 and 4 > 3", "(((1 + 2) == 3) and (4 > 3))")]
#[case("[1] ++ [2] == [1 2]", "(([1] ++ [2]) == [1 2])")]
#[case("1 mod 2 ** 3 // 4", "((1 mod (2 ** 3)) // 4)")]
#[case("$a in [1] or $b not-in [2]", "(($a in [1]) or ($b not-in [2]))")]
#[case("2 == null", "(2 == null)")]
#[case("2 != null", "(2 != null)")]
fn precedence(#[case] src: &str, #[case] expected: &str) {
    let ast = ok(src);
    assert_eq!(sexpr(src, last_expr(&ast)), expected);
}

#[rstest]
#[case("1 ^ 2", "**")]
#[case("1 pow 2", "**")]
#[case("1 % 2", "mod")]
#[case("1 === 2", "==")]
#[case("1 is 2", "==")]
#[case("[1] contains 1", "has")]
#[case("1 bits-and 2", "bit-and")]
#[case("1 << 2", "bit-shl")]
#[case("true ! false", "not")]
fn unknown_operators_get_help(#[case] src: &str, #[case] hint: &str) {
    let text = err_text(src);
    assert!(text.contains("unknown operator") && text.contains(hint), "{src}: {text}");
}

// --- redirections (test_parser.rs: test_redirection_*) ---------------------------------------

#[rstest]
#[case("let a = 1 err> /dev/null", RedirectionOperator::Err)]
#[case("let a = 1 out> /dev/null", RedirectionOperator::Out)]
#[case("let a = 1 out+err> /dev/null", RedirectionOperator::OutErr)]
#[case("mut a = 1 err> /dev/null", RedirectionOperator::Err)]
#[case("mut a = 1 out> /dev/null", RedirectionOperator::Out)]
#[case("mut a = 1 out+err> /dev/null", RedirectionOperator::OutErr)]
fn redirection_inside_let_value(#[case] src: &str, #[case] op: RedirectionOperator) {
    let ast = ok(src);
    let element = &ast.block.pipelines[0].elements[0];
    assert!(element.redirection.is_none(), "the redirection belongs to the value");
    let binding = match &element.expr.expr {
        Expr::Let(b) | Expr::Mut(b) => b,
        other => panic!("{other:?}"),
    };
    let inner = &binding.value.as_ref().unwrap().pipelines[0].elements[0];
    match inner.redirection.as_ref().unwrap() {
        PipelineRedirection::Single { target: RedirectionTarget::File { op: actual, .. }, .. } => {
            assert_eq!(actual.item, op)
        }
        other => panic!("{other:?}"),
    }
}

#[rstest]
#[case("o>")]
#[case("o>>")]
#[case("e>")]
#[case("e>>")]
#[case("o+e>")]
#[case("o+e>>")]
#[case("e>|")]
#[case("o+e>|")]
#[case("|o>")]
#[case("|e>|")]
#[case("e> file")]
#[case("o>> file")]
#[case("o+e>> file")]
#[case("|o+e> file")]
fn redirecting_nothing_is_an_error(#[case] src: &str) {
    assert!(parse(src).is_err(), "{src}");
}

#[rstest]
#[case("let a o> file = 1")]
#[case("mut a o> file = 1")]
#[case("let a = }")]
#[case("mut a = | }")]
#[case("def a [n= (if ]")]
#[case("const foo = if t")]
#[case("$ a")]
#[case("{a:b}/")]
fn malformed_input_does_not_panic(#[case] src: &str) {
    let (ast, diagnostics) = parse_lenient(src, &ParseConfig::new());
    assert!(!diagnostics.is_empty(), "{src}");
    let _ = nu_winnow_parser::pretty::dump(&ast);
    let _ = nu_winnow_parser::flatten::flatten(&ast);
}

// --- pipelines, comments and statements (repl/test_parser.rs) -----------------------------

#[rstest]
#[case("[1,2,3] | #comment\neach { |$it| $it + 2 } | # foo\nmath sum #bar", 3, 3)]
#[case("[1,2,3] #comment\n| #comment2\neach { |$it| $it + 2 } #foo\n| # bar\nmath sum #baz", 3, 5)]
#[case("[1,2,3] | #comment\n#comment2\neach { |$it| $it + 2 } #foo\n| # bar\n#baz\nmath sum #foobar", 3, 6)]
#[case("[[name, present]; [abc, true], [def, false]]\n# | where not present\n| get name.0", 2, 1)]
fn comments_between_pipeline_elements(#[case] src: &str, #[case] elements: usize, #[case] comments: usize) {
    let ast = ok(src);
    assert_eq!(ast.block.pipelines.len(), 1, "{src}");
    assert_eq!(ast.block.pipelines[0].elements.len(), elements, "{src}");
    assert_eq!(ast.comments.len(), comments, "{src}");
}

#[test]
fn hash_without_preceding_space_is_not_a_comment() {
    let ast = ok("echo test#testing");
    match &last_expr(&ast).expr {
        Expr::Call(c) => assert_eq!(string_value(c.positional_iter().next().unwrap()).0, "test#testing"),
        other => panic!("{other:?}"),
    }
    assert!(ok("# command_bar_text: { fg: '#C4C9C6' },").block.pipelines.is_empty());
}

#[test]
fn let_after_pipe_is_a_statement() {
    let ast = ok("ls | let files");
    let els = &ast.block.pipelines[0].elements;
    assert_eq!(els.len(), 2);
    assert!(matches!(els[1].expr.expr, Expr::Let(_)));
}

#[test]
fn empty_braces_as_row_condition() {
    let ast = ok("[0 1 2] | where {}");
    match &last_expr(&ast).expr {
        Expr::Where(w) => assert!(matches!(w.condition.expr, Expr::Closure(_))),
        other => panic!("{other:?}"),
    }
}

#[test]
fn datetime_in_record_value() {
    let ast = ok("{ a: 2024-07-23T22:54:54.532100627+02:00 b:xy }");
    match &last_expr(&ast).expr {
        Expr::Record(items) => match &items[0] {
            RecordItem::Pair { value, .. } => assert!(matches!(value.expr, Expr::DateTime(_))),
            other => panic!("{other:?}"),
        },
        other => panic!("{other:?}"),
    }
}

#[rstest]
#[case("{ :: x }", "key")]
#[case("{ a: x:y }", "value")]
#[case("{ a: x('y'):z }", "value")]
#[case("{a: http://x.y}", "value")]
fn bare_colons_in_records_are_refused(#[case] src: &str, #[case] position: &str) {
    let text = err_text(src);
    assert!(text.contains(&format!("colon in bare word specifying record {position}")), "{src}: {text}");
}

#[rstest]
#[case("{ ;: x }")]
#[case("{ a: || }")]
#[case("{ show_banner: false; table_mode: rounded }")]
#[case("{ a: 2 b }")]
#[case("{ a: 2 b 3 }")]
#[case("{ a: 2 b: }")]
fn confusing_records_are_refused(#[case] src: &str) {
    assert!(parse(src).is_err(), "{src}");
}

#[test]
fn attribute_values() {
    // nu's test registers an `attr echo` command; here it is declared in the source.
    let ast = ok("def \"attr echo\" [x] { $x }\n@echo \"hello world\"\n@echo 42\ndef foo [] {}");
    match &last_expr(&ast).expr {
        Expr::AttributeBlock(a) => {
            assert_eq!(a.attributes.len(), 2);
            assert_eq!(a.attributes[0].name.item, "echo");
            assert_eq!(string_value(a.attributes[0].arguments.first().map(arg_expr).unwrap()).0, "hello world");
            assert!(matches!(arg_expr(&a.attributes[1].arguments[0]).expr, Expr::Int(42)));
            assert!(matches!(a.item.expr, Expr::Def(_)));
        }
        other => panic!("{other:?}"),
    }
}

fn arg_expr<'a>(arg: &'a Argument<'a>) -> &'a Expression<'a> {
    match arg {
        Argument::Positional(e) => e,
        other => panic!("{other:?}"),
    }
}

#[rstest]
#[case("def test [ --a: any = 32 ] {}")]
#[case("def test [ --a: number = 32 ] {}")]
#[case("def test [ --a: number = 32.0 ] {}")]
#[case("def test [ --a: list<any> = [ 1 2 3 ] ] {}")]
#[case("def test [ --a: record<a: int b: string> = { a: 32 b: 'qwe' c: 'wqe' } ] {}")]
#[case("def test [ --a: record<a: any b: any> = { a: 32 b: 'qwe'} ] {}")]
#[case("def test []: int -> int { 1 }")]
#[case("def test []: string -> string { 'qwe' }")]
#[case("def test []: nothing -> nothing { null }")]
#[case("def test []: list<string> -> list<string> { [] }")]
#[case("def test []: record<a: int b: int> -> record<c: int e: int> { {c: 1 e: 1} }")]
#[case("def test []: table<a: int b: int> -> table<c: int e: int> { [ {c: 1 e: 1} ] }")]
#[case("def test []: nothing -> record<c: int e: int> { {c: 1 e: 1} }")]
#[case("extern cmd [in, --env]")]
#[case("export extern cmd (in, --env, ...nu)")]
#[case("extern cmd [in: bool=true]")]
#[case("def foo [--verbose(-v), # my test flag\n ...rest: int # my rest comment\n] { }")]
fn signature_forms_from_nushell_tests(#[case] src: &str) {
    let ast = ok(src);
    assert!(matches!(last_expr(&ast).expr, Expr::Def(_) | Expr::Extern(_) | Expr::Export(_)));
}

/// `in`, `nu`, `env` and `ans` are built-in variables: a parameter, a `let`
/// or a match pattern may not declare them (`tests/parsing/mod.rs` in nushell).
#[rstest]
#[case("def test [ in ] {}")]
#[case("def test [ in: string ] {}")]
#[case("def test [ --in (-i): list<any> ] {}")]
#[case("def test [ env, in, nu ] {}")]
#[case("def test [ --env ] {}")]
#[case("let in = 1")]
#[case("mut nu = 1")]
#[case("const ans = 1")]
#[case("for env in [] {}")]
#[case("do {|in| }")]
#[case("match 1 { $in => 1 }")]
#[case("match {a: 1} { {a: $env} => 1 }")]
fn reserved_variable_names_are_errors(#[case] src: &str) {
    assert!(err_text(src).contains("used as variable name"), "{src}");
}

#[rstest]
#[case("let a: int = 1", SyntaxShape::Int)]
#[case("let a: string = 'qwe'", SyntaxShape::String)]
#[case("let a: nothing = null", SyntaxShape::Nothing)]
#[case("let a: list<string> = []", SyntaxShape::List(None))]
#[case("let a: record<a: int b: int> = {a: 1 b: 1}", SyntaxShape::Record(Vec::new()))]
#[case("let a: table<a: int b: int> = [[a b]; [1 2] [3 4]]", SyntaxShape::Table(Vec::new()))]
fn let_type_annotations(#[case] src: &str, #[case] kind: SyntaxShape<'static>) {
    let ast = ok(src);
    match &last_expr(&ast).expr {
        Expr::Let(b) => {
            let actual = &b.ty.as_ref().unwrap().shape;
            assert_eq!(std::mem::discriminant(actual), std::mem::discriminant(&kind), "{src}: {actual:?}");
        }
        other => panic!("{other:?}"),
    }
}

#[rstest]
#[case("let a int = 1")]
#[case("mut a record<a: int b: int> = {a: 1 b: 1}")]
#[case("const a string = 'Hello World\n'")]
#[case("let a : int = 1")]
fn let_type_without_colon_is_extra_tokens(#[case] src: &str) {
    let e = parse(src).unwrap_err();
    assert!(matches!(e.primary().kind, ErrorKind::ExtraTokens | ErrorKind::Expected(_)), "{src}: {e}");
}

#[rstest]
#[case("let $\"foo bar\" = 4")]
#[case("let $foo-bar = 4")]
#[case("let $=foo = 4")]
#[case("let = if $")]
#[case("mut = if $")]
#[case("const = if $")]
#[case("for in")]
#[case("def foo3 [-l?:int] { $l }")]
#[case("extern cmd[]")]
#[case("extern cmd(--flag)")]
#[case("def def [] {}")]
#[case("def def (=a|s)>")]
#[case("def a [] (echo 4)")]
#[case("def foo []: int { 3 }")]
#[case("def foo []: int -> { 3 }")]
#[case("def foo []: int -> int@completer {}")]
#[case("let x: int@completer = 42")]
#[case("if true { || print hi }")]
#[case("if true { |x| $x }")]
fn declaration_errors_from_nushell_tests(#[case] src: &str) {
    assert!(parse(src).is_err(), "{src}");
}

// --- the lexer (test_lex.rs) ---------------------------------------------------------------

fn kinds_and_texts(src: &str, opts: LexOptions) -> Vec<(TokenContents, &str)> {
    lex(src, 0, opts).unwrap().into_iter().map(|t| (t.contents, t.text(src))).collect()
}

#[test]
fn lex_newline_and_semicolon() {
    let src = "let x = 300\nlet y = 500;";
    let toks = lex(src, 0, LexOptions::BLOCK).unwrap();
    assert!(toks.iter().any(|t| t.contents == TokenContents::Eol && t.span == Span::new(11, 12)));
    assert_eq!(toks[toks.len() - 2].contents, TokenContents::Semicolon);
}

#[rstest]
#[case("items: list<string>", &["items", ":", "list<string>"])]
#[case("config: record<name: string>", &["config", ":", "record<name: string>"])]
#[case("items: list<>", &["items", ":", "list<>"])]
#[case("items: list <string>", &["items", ":", "list", "<string>"])]
#[case("items: list< string>", &["items", ":", "list< string>"])]
#[case("items: list<string >", &["items", ":", "list<string >"])]
#[case("items: list< string >", &["items", ":", "list< string >"])]
#[case("items: list<record<name: string>>", &["items", ":", "list<record<name: string>>"])]
#[case("x: int = 1, --flag(-f)", &["x", ":", "int", "=", "1", ",", "--flag(-f)"])]
fn lex_signatures(#[case] src: &str, #[case] expected: &[&str]) {
    let texts: Vec<&str> = kinds_and_texts(src, LexOptions::SIGNATURE).iter().map(|t| t.1).collect();
    let texts = &texts[..texts.len() - 1];
    assert_eq!(texts, expected, "{src}");
}

#[rstest]
#[case("items: list<record<name: string>", ">")]
#[case("items: list<string", ">")]
fn lex_unterminated_type_annotations(#[case] src: &str, #[case] delimiter: &str) {
    let e = lex(src, 0, LexOptions::SIGNATURE).unwrap_err();
    assert!(matches!(e.kind, ErrorKind::Unclosed { delimiter: d, .. } if d == delimiter), "{src}: {e}");
}

#[test]
fn lex_empty_input_is_just_eof() {
    let toks = lex("", 0, LexOptions::BLOCK).unwrap();
    assert_eq!(toks.len(), 1);
    assert_eq!(toks[0].contents, TokenContents::Eof);
}

#[test]
fn lex_parenthesised_expression_is_one_item() {
    let toks = lex("let x = (300 + (322 * 444));", 0, LexOptions::BLOCK).unwrap();
    assert_eq!((toks[3].contents, toks[3].span), (TokenContents::Item, Span::new(8, 27)));
}

#[test]
fn lex_comment_spans() {
    let toks = lex("let x = 300 # a comment \n $x + 444", 0, LexOptions::BLOCK).unwrap();
    assert_eq!((toks[4].contents, toks[4].span), (TokenContents::Comment, Span::new(12, 24)));

    let src = "let z = 42 #the comment \n let x#y = 69 #hello \n let flk = nixpkgs#hello #hello";
    let toks = lex(src, 0, LexOptions::BLOCK).unwrap();
    assert_eq!((toks[4].contents, toks[4].span), (TokenContents::Comment, Span::new(11, 24)));
    assert_eq!((toks[7].contents, toks[7].span), (TokenContents::Item, Span::new(30, 33)));
    assert_eq!((toks[10].contents, toks[10].span), (TokenContents::Comment, Span::new(39, 46)));
    assert_eq!((toks[15].contents, toks[15].span), (TokenContents::Item, Span::new(58, 71)));
    assert_eq!((toks[16].contents, toks[16].span), (TokenContents::Comment, Span::new(72, 78)));

    // Comments keep the end-of-line token after them.
    let src = "let z = 4 #comment \n let x = 4 # comment\n let y = 1 # comment";
    let toks = lex(src, 0, LexOptions::BLOCK).unwrap();
    assert_eq!((toks[4].contents, toks[4].span), (TokenContents::Comment, Span::new(10, 19)));
    assert_eq!((toks[5].contents, toks[5].span), (TokenContents::Eol, Span::new(19, 20)));
    assert_eq!((toks[10].contents, toks[10].span), (TokenContents::Comment, Span::new(31, 40)));
    assert_eq!((toks[11].contents, toks[11].span), (TokenContents::Eol, Span::new(40, 41)));
}

#[test]
fn lex_hash_inside_brackets() {
    assert!(lex("1..10 | each {echo test#testing }", 0, LexOptions::BLOCK).is_ok());
    for src in ["1..10 | each {echo test #testing }", "1..10 | each {echo test\t#testing }"] {
        let e = lex(src, 0, LexOptions::BLOCK).unwrap_err();
        assert!(
            matches!(e.kind, ErrorKind::Unclosed { delimiter: "}", open } if open == Span::new(13, 14)),
            "{src}: {e}"
        );
    }
}

#[rstest]
#[case("let x = (300 + ( 4 + 1)", ")", 8)]
#[case("let x = '300 + 4 + 1", "'", 8)]
#[case("print (1 + 2", ")", 6)]
#[case("let x = [1, 2", "]", 8)]
#[case("let y = [1, 2, 3\n4, 5, 6", "]", 8)]
#[case("let n = (\n  1 + 2", ")", 8)]
#[case("let r = {\n  a: 1\n  b: 2", "}", 8)]
// The innermost open delimiter is reported: the `{` after `ls:`.
#[case("$env.config = {\n  ls: {\n    use_ls_colors: true\n", "}", 22)]
#[case("$\"('\"", ")", 2)]
#[case("$\"(1", ")", 2)]
#[case("$\"foo (2 + 3\"", ")", 6)]
fn lex_unclosed_delimiters_point_at_the_opener(#[case] src: &str, #[case] delimiter: &str, #[case] open_at: usize) {
    let e = lex(src, 0, LexOptions::BLOCK).unwrap_err();
    match e.kind {
        ErrorKind::Unclosed { delimiter: d, open } => {
            assert_eq!(d, delimiter, "{src}");
            assert_eq!(open, Span::new(open_at, open_at + 1), "{src}");
        }
        other => panic!("{src}: {other:?}"),
    }
}

#[rstest]
#[case("[1, 2, 3)", ")", "[")]
#[case("(1, 2]", "]", "(")]
#[case("{ a: 1 )", ")", "{")]
#[case("}", "}", "{")]
#[case(")", ")", "(")]
fn lex_mismatched_closers(#[case] src: &str, #[case] found: &str, #[case] expected: &str) {
    let e = lex(src, 0, LexOptions::BLOCK).unwrap_err();
    assert!(
        matches!(e.kind, ErrorKind::Unbalanced { found: f, expected: x } if f == found && x == expected),
        "{src}: {e}"
    );
}

#[rstest]
#[case(r#"$"('" "')""#)]
#[case(r#"$"('a' + "b")""#)]
#[case(r#"$'("a b")'"#)]
#[case(r#"$"(1 + (2 * 3)) end""#)]
#[case(r#"$"\('not an expr'\)""#)]
#[case(r#"$"("a\"b")""#)]
#[case(r#"$"a(1)b(2)""#)]
#[case(r#"$"($"in(2)ner")""#)]
#[case(r#"$'($"a" + $'b')'"#)]
fn lex_interpolation_is_one_item(#[case] src: &str) {
    let toks = lex(src, 0, LexOptions::BLOCK).unwrap();
    let items: Vec<_> = toks.iter().filter(|t| t.contents == TokenContents::Item).collect();
    assert_eq!(items.len(), 1, "{src}: {toks:?}");
    assert_eq!(items[0].span, Span::new(0, src.len()));
}

#[rstest]
#[case("def f [] {\n        let emoji_dict = ({\n        \"200\": \"x\",\n    })\n}\n")]
#[case(
    "{||\n    if $in < 1hr {\n      'red'\n      } else if $in < 1wk {\n      'green'\n    } else if $in < 6wk {\n      'blue'\n    } else { 'gray' }\n  }\n"
)]
#[case("$env.config = {\n  hooks: {\n    pre: 1\n  }\n  rm: {\n    x: 1\n  }\n}\n")]
#[case("let x = [1 (2 + 3) {a: 4}]\n")]
#[case("def f [a: int, b: string] { $a + ($b | str length) }\n")]
#[case("ls\n| where type == file\n| get name\n")]
#[case("ls |\n  get name\n")]
#[case("{ \"type\": 1, name: 2 }\n")]
#[case("let x = \"hello\nworld\"\n")]
#[case("let x = 'hello\nworld'\n")]
#[case("let y = [1, 2, 3\n4, 5, 6]\n")]
#[case("let y = [\n  1,\n  (2 + 3),\n  {a: 4}\n]\n")]
#[case("let r = {\n  a: 1\n  b: 2\n}\n")]
#[case("let n = (\n  1 + 2\n)\n")]
fn valid_layouts_never_get_delimiter_errors(#[case] src: &str) {
    assert!(lex(src, 0, LexOptions::BLOCK).is_ok(), "{src}");
    ok(src);
}

#[test]
fn lex_large_nested_record_completes() {
    let mut src = String::from("$env.config = {\n");
    for i in 0..2000 {
        src.push_str(&format!("  key{i}: {{\n    nested: {i}\n  }}\n"));
    }
    src.push('}');
    assert!(lex(&src, 0, LexOptions::BLOCK).is_ok());
    let ast = ok(&src);
    match &last_expr(&ast).expr {
        Expr::Assignment(a) => match &a.rhs.pipelines[0].elements[0].expr.expr {
            Expr::Record(items) => assert_eq!(items.len(), 2000),
            other => panic!("{other:?}"),
        },
        other => panic!("{other:?}"),
    }
}

#[test]
fn deeply_nested_lists_do_not_blow_up() {
    let e = parse("[[[[[[[[[[[[[[[[[[[[[[[[[[[[").unwrap_err();
    assert!(matches!(e.primary().kind, ErrorKind::Unclosed { delimiter: "]", .. }));
    let ast = ok("[[[[[[[[[[[[[[[[[[[[[[[[[[[[1]]]]]]]]]]]]]]]]]]]]]]]]]]]]");
    assert!(matches!(last_expr(&ast).expr, Expr::List(_)));
}

#[test]
fn deeply_nested_modules_do_not_blow_up() {
    let src = "module foo { ".repeat(28) + "use bar.nu " + &"}".repeat(28);
    let ast = ok(&src);
    assert!(matches!(last_expr(&ast).expr, Expr::Module(_)));
}

// --- the crate's own corpus of Nushell repository samples -------------------------------------

/// `tests/parsing/samples` in the Nushell repository, when a checkout is
/// available next to this one; skipped otherwise.
#[test]
fn nushell_parsing_samples_parse() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/parsing/samples");
    let Ok(entries) = std::fs::read_dir(&dir) else {
        eprintln!("no Nushell checkout at {}; skipping", dir.display());
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "nu") {
            let src = std::fs::read_to_string(&path).unwrap();
            ok(&src);
        }
    }
}
