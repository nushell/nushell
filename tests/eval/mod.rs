use nu_test_support::{nu, playground::Playground};
use regex::Regex;

#[test]
fn record_with_redefined_key() {
    let actual = nu!("{x: 1, x: 2}");

    assert!(actual.err.contains("redefined"));
}

#[test]
fn run_file_parse_error() {
    let actual = nu!(
        cwd: "tests/fixtures/eval",
        "nu script.nu"
    );

    assert!(actual.err.contains("unknown type"));
}

enum ExpectedOut<'a> {
    /// Equals a string exactly
    Eq(&'a str),
    /// Matches a regex
    Matches(&'a str),
    /// Drops a file that contains these contents
    FileEq(&'a str, &'a str),
}
use self::ExpectedOut::*;

fn test_eval(source: &str, expected_out: ExpectedOut) {
    Playground::setup("test_eval_ast", |ast_dirs, _playground| {
        Playground::setup("test_eval_ir", |ir_dirs, _playground| {
            let actual_ast = nu!(
                cwd: ast_dirs.test(),
                use_ir: false,
                source,
            );
            let actual_ir = nu!(
                cwd: ir_dirs.test(),
                use_ir: true,
                source,
            );

            assert!(actual_ast.status.success());
            assert!(actual_ir.status.success());
            match expected_out {
                Eq(eq) => {
                    assert_eq!(actual_ast.out, eq);
                    assert_eq!(actual_ir.out, eq);
                }
                Matches(regex) => {
                    let compiled_regex = Regex::new(regex).expect("regex failed to compile");
                    assert!(
                        compiled_regex.is_match(&actual_ast.out),
                        "AST eval out does not match: {}",
                        regex
                    );
                    assert!(
                        compiled_regex.is_match(&actual_ir.out),
                        "IR eval out does not match: {}",
                        regex
                    );
                }
                FileEq(path, contents) => {
                    let ast_contents = std::fs::read_to_string(ast_dirs.test().join(path))
                        .expect("failed to read AST file");
                    let ir_contents = std::fs::read_to_string(ir_dirs.test().join(path))
                        .expect("failed to read IR file");
                    assert_eq!(ast_contents.trim(), contents);
                    assert_eq!(ir_contents.trim(), contents);
                }
            }
            assert_eq!(actual_ast.out, actual_ir.out);
        })
    });
}

#[test]
fn literal_bool() {
    test_eval("true", Eq("true"))
}

#[test]
fn literal_int() {
    test_eval("1", Eq("1"))
}

#[test]
fn literal_float() {
    test_eval("1.5", Eq("1.5"))
}

#[test]
fn literal_binary() {
    test_eval("0x[1f 2f f0]", Matches("Length.*1f.*2f.*f0"))
}

#[test]
fn literal_closure() {
    test_eval("{||}", Matches("<Closure"))
}

#[test]
fn literal_range() {
    test_eval("0..2..10", Matches("10"))
}

#[test]
fn literal_list() {
    test_eval("[foo bar baz]", Matches("foo.*bar.*baz"))
}

#[test]
fn literal_record() {
    test_eval("{foo: bar, baz: quux}", Matches("foo.*bar.*baz.*quux"))
}

#[test]
fn literal_string() {
    test_eval(r#""foobar""#, Eq("foobar"))
}

#[test]
fn literal_raw_string() {
    test_eval(r#"r#'bazquux'#"#, Eq("bazquux"))
}

#[test]
fn literal_nothing() {
    test_eval("null", Eq(""))
}

#[test]
fn list_spread() {
    test_eval("[foo bar ...[baz quux]] | length", Eq("4"))
}

#[test]
fn record_spread() {
    test_eval("{foo: bar ...{baz: quux}} | columns | length", Eq("2"))
}

#[test]
fn binary_op_example() {
    test_eval(
        "(([1 2] ++ [3 4]) == [1 2 3 4]) and (([1 2 3] ++ 4) == ([1] ++ [2 3 4]))",
        Eq("true"),
    )
}

#[test]
fn range_from_expressions() {
    test_eval("(1 + 1)..(2 + 2)", Matches("2.*3.*4"))
}

#[test]
fn list_from_expressions() {
    test_eval(
        "[('foo' | str upcase) ('BAR' | str downcase)]",
        Matches("FOO.*bar"),
    )
}

#[test]
fn record_from_expressions() {
    test_eval("{('foo' | str upcase): 42}", Matches("FOO.*42"))
}

#[test]
fn external_call() {
    test_eval("nu --testbin cococo foo=bar baz", Eq("foo=bar baz"))
}

#[test]
fn external_call_redirect_pipe() {
    test_eval(
        "nu --testbin cococo foo=bar baz | str upcase",
        Eq("FOO=BAR BAZ"),
    )
}

#[test]
fn external_call_redirect_file() {
    test_eval(
        "nu --testbin cococo hello out> hello.txt",
        FileEq("hello.txt", "hello"),
    )
}
