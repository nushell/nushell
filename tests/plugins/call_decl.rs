use nu_test_support::prelude::*;

#[test]
#[deps(NU_PLUGIN_EXAMPLE)]
fn call_to_json() -> Result {
    test()
        .run("[42] | example call-decl 'to json' {indent: 4}")
        .expect_value_eq("[\n    42\n]")
}

#[test]
#[deps(NU_PLUGIN_EXAMPLE)]
fn call_reduce() -> Result {
    test()
        .run("[1 2 3] | example call-decl 'reduce' {fold: 10} {|it, acc| $it + $acc}")
        .expect_value_eq(16)
}

#[test]
#[deps(NU_PLUGIN_EXAMPLE)]
fn call_scope_variables() -> Result {
    let code = "
        let test_var = 10
        example call-decl 'scope variables' 
        | where name == '$test_var' 
        | length
    ";

    test().run(code).expect_value_eq(1)
}
