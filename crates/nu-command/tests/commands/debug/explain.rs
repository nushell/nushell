use nu_test_support::prelude::*;
use rstest::rstest;

// `explain` evaluates each argument of a call. A keyword can hold another call as an argument,
// and that call is only compiled as part of the keyword, so it shows as an error value. A
// subexpression is only compiled as part of the closure, so it shows as its source text and the
// type the parser gave it.

#[test]
fn explain_else_if_shows_the_nested_if_as_an_error() -> Result {
    test()
        .run("explain { if true { 1 } else if false { 2 } } | get 0.cmd_args.type")
        .expect_value_eq(["bool", "nothing", "error"])
}

#[test]
fn explain_alias_does_not_run_the_aliased_command(playground: Playground) -> Result {
    let code = "
        let args = explain { alias mk = touch marker.txt } | get 0.cmd_args.type
        {args: $args, ran: ('marker.txt' | path exists)}
    ";

    test()
        .cwd(playground.path())
        .run(code)
        .expect_value_eq(test_value!({
            args: ["string", "error"],
            ran: false,
        }))
}

#[rstest]
#[case::subexpression("explain { append (1 + 2) }", "(1 + 2)", "int")]
#[case::interpolation(r#"explain { append $"a(1 + 2)b" }"#, r#"$"a(1 + 2)b""#, "string")]
#[case::list("explain { append [(3)] }", "[(3)]", "list<int>")]
#[case::cell_path("explain { append (ls).name }", "(ls).name", "any")]
// A closure isn't run when it's evaluated, so a subexpression in its body doesn't count.
#[case::closure("explain { each { (1 + 2) } }", "{ (1 + 2) }", "closure")]
fn explain_shows_an_argument_with_a_subexpression_as_source(
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
