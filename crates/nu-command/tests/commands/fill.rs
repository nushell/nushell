use nu_test_support::prelude::*;

#[test]
fn string_fill_plain() -> Result {
    test()
        .run(r#""abc" | fill --alignment center --character "+" --width 5"#)
        .expect_value_eq("+abc+")
}

#[test]
fn string_fill_fancy() -> Result {
    let code = r#"
        $"(ansi red)a(ansi green)\u{65}\u{308}(ansi cyan)c(ansi reset)"
        | fill --alignment center --character "+" --width 5
    "#;

    test()
        .run(code)
        .expect_value_eq("+\u{1b}[31ma\u{1b}[32me\u{308}\u{1b}[36mc\u{1b}[0m+")
}
