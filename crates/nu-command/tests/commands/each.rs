use nu_test_support::prelude::*;

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

    let eval_block_with_input_count = format!("{err:?}").matches("EvalBlockWithInput").count();
    assert_eq!(eval_block_with_input_count, 2);
    Ok(())
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
