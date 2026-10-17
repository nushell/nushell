use nu_test_support::prelude::*;
use pretty_assertions::assert_matches;
use rstest::rstest;

#[test]
fn each_works_separately() -> Result {
    test()
        .run("[1 2 3] | each { |it| echo $it 10 | math sum }")
        .expect_value_eq([11, 12, 13])
}

#[test]
fn each_group_works() -> Result {
    test()
        .run("[1 2 3 4 5 6] | chunks 3")
        .expect_value_eq([[1, 2, 3], [4, 5, 6]])
}

#[test]
fn each_window() -> Result {
    test()
        .run("[1 2 3 4] | window 3")
        .expect_value_eq([[1, 2, 3], [2, 3, 4]])
}

#[test]
fn each_window_stride() -> Result {
    test()
        .run("[1 2 3 4 5 6] | window 3 -s 2")
        .expect_value_eq([[1, 2, 3], [3, 4, 5]])
}

#[test]
fn each_no_args_in_block() -> Result {
    test()
        .run("[[foo bar]; [a b] [c d] [e f]] | each {|i| $i } | get 1")
        .expect_value_eq(test_record! {
            "foo" => "c",
            "bar" => "d",
        })
}

#[test]
#[deps(TESTBIN_COCOCO)]
fn each_implicit_it_in_block() -> Result {
    test()
        .run("[[foo bar]; [a b] [c d] [e f]] | each { |it| cococo $it.foo } | str join")
        .expect_value_eq("ace")
}

#[test]
fn each_uses_enumerate_index() -> Result {
    test()
        .run("[7 8 9 10] | enumerate | each {|el| $el.index }")
        .expect_value_eq([0, 1, 2, 3])
}

#[test]
fn each_while_uses_enumerate_index() -> Result {
    test()
        .run("[7 8 9 10] | enumerate | each while {|el| $el.index }")
        .expect_value_eq([0, 1, 2, 3])
}

#[test]
fn errors_in_nested_each_show() -> Result {
    let err = test()
        .run("[[1,2]] | each {|x| $x | each {|y| error make {msg: \"oh noes\"} } }")
        .expect_shell_error()?;

    assert_contains("oh noes", format!("{err:?}"));
    Ok(())
}

#[test]
fn errors_in_nested_each_full_chain() -> Result {
    let err = test()
        .run("0..1 | each {|i| 0..1 | each {|j| error make {msg: boom} } }")
        .expect_shell_error()?;

    assert_matches!(err, ShellError::EvalBlockWithInput(..));
    let err = err.into_inner()?;
    assert_matches!(err, ShellError::EvalBlockWithInput(..));
    assert_eq!(err.into_inner()?.into_labeled()?.msg, "boom");
    Ok(())
}

#[rstest]
#[case::each("[1] | each { error make {msg: inner} }")]
#[case::nested_each("0..1 | each {|i| 0..1 | each {|j| error make {msg: inner} } }")]
#[case::par_each("[1] | par-each { error make {msg: inner} }")]
#[case::items("{a: 1} | items {|k, v| error make {msg: inner} }")]
fn closure_error_message_reaches_catch(#[case] code: &str) -> Result {
    test()
        .run(format!("try {{ {code} }} catch {{|e| $e.msg }}"))
        .expect_value_eq("inner")
}

#[test]
fn closure_error_keeps_its_code() -> Result {
    test()
        .run("[1 2] | each {|x| $x / 0 }")
        .expect_error_code_eq("nu::shell::division_by_zero")
}

#[test]
fn closure_error_labels_the_input_value() -> Result {
    let code = "
        try { [1] | each { error make {msg: inner} } } catch {|e| $e.details.labels.text }
    ";
    test().run(code).expect_value_eq(["", "source value"])
}

/// A closure error with its own source text (here the XML) renders as a related report, so the
/// input value's label stays in the code's snippet and the XML error keeps its own snippet.
#[test]
fn closure_error_with_own_source_keeps_both_snippets() -> Result {
    let code = r#"
        try { ["<a><b></a>"] | each { from xml } } catch {|e| $e.rendered | ansi strip }
    "#;
    let rendered: String = test().run(code)?;
    let reports: Vec<_> = rendered.split("Error: ").skip(1).collect();
    assert_eq!(reports.len(), 2, "{rendered}");
    let (input, xml) = (reports[0], reports[1]);
    assert_contains("each { from xml }", input);
    assert_contains("source value", input);
    assert_contains("Unexpected close tag a, expected b", xml);
    assert_contains_not("each { from xml }", xml);
    Ok(())
}

/// Nested closures label only the innermost input value, the one the failing closure ran on.
#[rstest]
#[case::shared_input("let xs = [1]; $xs | each { $xs | each { error make {msg: inner} } }")]
#[case::different_inputs("[1] | each { [2] | each { error make {msg: inner} } }")]
fn nested_closure_errors_label_one_input_value(#[case] code: &str) -> Result {
    test()
        .run(format!(
            "try {{ {code} }} catch {{|e| $e.details.labels.text }}"
        ))
        .expect_value_eq(["", "source value"])
}

#[test]
fn each_noop_on_single_null() -> Result {
    test().run("null | each { \"test\" }").expect_value_eq(())
}

#[test]
fn each_flatten_dont_collect() -> Result {
    Playground::setup("each_flatten_dont_collect", |dirs, _sandbox| {
        let collected = r##"
            '' | save order.txt
            [0 3]
            | each {|e| $e..<($e + 3) | each {|e| $"\(($e)\)" | save --append order.txt; $e } }
            | flatten
            | each {|e| $"[($e)]" | save --append order.txt; $e }
            | ignore
            open order.txt
        "##;

        test()
            .cwd(dirs.test())
            .run(collected)
            .expect_value_eq("(0)(1)(2)[0][1][2](3)(4)(5)[3][4][5]")?;

        let streamed = r##"
            '' | save --force order.txt
            [0 3]
            | each --flatten {|e| $e..<($e + 3) | each {|e| $"\(($e)\)" | save --append order.txt; $e } }
            | each {|e| $"[($e)]" | save --append order.txt; $e }
            | ignore
            open order.txt
        "##;

        test()
            .cwd(dirs.test())
            .run(streamed)
            .expect_value_eq("(0)[0](1)[1](2)[2](3)[3](4)[4](5)[5]")
    })
}
