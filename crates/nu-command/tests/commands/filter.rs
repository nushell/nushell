use nu_test_support::prelude::*;

#[test]
#[deps(NU)]
fn filter_with_return_in_closure() -> Result {
    let code = "nu -n -c '
        1..10 | filter { |it|
            if $it mod 2 == 0 {
                return true
            };
            return false;
        } | to nuon
    ' | complete";
    let result: CompleteResult = test().run(code)?;

    assert_eq!(result.stdout.trim(), "[2, 4, 6, 8, 10]");
    assert_contains("deprecated", result.stderr);
    Ok(())
}
