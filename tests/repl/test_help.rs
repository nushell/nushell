use crate::repl::tests::{run_test, TestResult};
use rstest::rstest;

#[rstest]
// avoid feeding strings containing parens to regex.  Does not end well.
#[case(": arga help")]
#[case("argb help")]
#[case("optional, default: 20")]
#[case(": f1 switch")]
#[case(": f2 named no default")]
#[case(": f3 named default 3")]
#[case("default: 33")]
#[case("--help: Display the help message")]
fn can_get_help(#[case] exp_result: &str) -> TestResult {
    run_test(
        &format!(
            r#"def t [a:string, # arga help
            b:int=20, # argb help
            --f1, # f1 switch help
            --f2:string, # f2 named no default
            --f3:int=33 # f3 named default 3
            ] {{ true }};
            help t | ansi strip | find `{exp_result}` | get 0 | str replace --all --regex '^(.*({exp_result}).*)$' '$2'"#,
        ),
        exp_result,
    )
}
