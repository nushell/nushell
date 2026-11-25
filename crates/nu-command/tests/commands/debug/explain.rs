use nu_test_support::prelude::*;
use rstest::rstest;

// `explain` shows each argument of a call without running any code. A bare variable or a
// constant shows its value. Anything else, such as a subexpression, a cell path on a variable or
// a call that a keyword takes as an argument, shows its source text and the type the parser gave
// it.

#[rstest]
#[case::captured_var(r#"let v = "abc"; explain { str length $v }"#, "abc", "string")]
#[case::const_var(r#"const c = "k"; explain { str length $c }"#, "k", "string")]
#[case::literal("explain { append 1kb }", "1 kB", "filesize")]
#[case::record_literal("explain { append {a: 1} }", "{a: 1}", "record<a: int>")]
#[case::spread_var(r#"let l = ["a" "b"]; explain { ls ...$l }"#, "[a b]", "list<string>")]
fn explain_shows_the_value_of_a_variable_or_constant(
    #[case] explain: &str,
    #[case] name: &str,
    #[case] ty: &str,
) -> Result {
    test()
        .run(format!("{explain} | get 0.cmd_args.0 | select name type"))
        .expect_value_eq(test_record! {
            "name" => name,
            "type" => ty,
        })
}

#[rstest]
#[case::subexpression("explain { append (1 + 2) }", "(1 + 2)", "int")]
#[case::interpolation(r#"explain { append $"a(1 + 2)b" }"#, r#"$"a(1 + 2)b""#, "string")]
#[case::list("explain { append [(3)] }", "[(3)]", "list<int>")]
#[case::cell_path("explain { append (ls).name }", "(ls).name", "any")]
// A closure isn't run when it's evaluated, so a subexpression in its body doesn't count.
#[case::closure("explain { each { (1 + 2) } }", "{ (1 + 2) }", "closure")]
// A closure's parameters and its own variables aren't on the stack while `explain` runs.
#[case::closure_parameter("explain {|x| echo $x }", "$x", "any")]
#[case::cell_path_on_var("let r = {a: 1}; explain { append $r.a }", "$r.a", "int")]
#[case::list_with_var("let v = 2; explain { append [$v 3] }", "[$v 3]", "list<int>")]
#[case::block("explain { loop { break } }", "{ break }", "block")]
#[case::row_condition("explain { where $it > 0 }", "$it > 0", "bool")]
fn explain_shows_an_argument_without_a_constant_value_as_source(
    #[case] explain: &str,
    #[case] name: &str,
    #[case] ty: &str,
) -> Result {
    test()
        .run(format!("{explain} | get 0.cmd_args.0 | select name type"))
        .expect_value_eq(test_record! {
            "name" => name,
            "type" => ty,
        })
}

// The nested `if` can run as a const command, so this fails if `explain` const-evaluates it.
#[test]
fn explain_shows_a_call_in_a_keyword_argument_as_source() -> Result {
    test()
        .run("explain { if true { 1 } else if false { 2 } } | get 0.cmd_args.2 | select name type")
        .expect_value_eq(test_record! {
            "name" => "if false { 2 }",
            "type" => "oneof<int, nothing>",
        })
}

#[test]
fn explain_alias_does_not_run_the_aliased_command(playground: Playground) -> Result {
    let code = "
        let args = explain { alias mk = touch marker.txt } | get 0.cmd_args.name
        {args: $args, ran: ('marker.txt' | path exists)}
    ";

    test()
        .cwd(playground.path())
        .run(code)
        .expect_value_eq(test_value!({
            args: ["mk", "touch marker.txt"],
            ran: false,
        }))
}

#[test]
fn explain_does_not_run_a_subexpression(playground: Playground) -> Result {
    let code = "
        let args = explain { append (touch marker.txt) } | get 0.cmd_args.name
        {args: $args, ran: ('marker.txt' | path exists)}
    ";

    test()
        .cwd(playground.path())
        .run(code)
        .expect_value_eq(test_value!({
            args: ["(touch marker.txt)"],
            ran: false,
        }))
}
