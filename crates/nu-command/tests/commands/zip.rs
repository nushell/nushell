use nu_test_support::prelude::*;

const ZIP_POWERED_TEST_ASSERTION_COMMAND: &str = "
export def expect [
    left,
    --to-eq,
    right
] {
    $left | zip $right | all {|row|
        $row.name.0 == $row.name.1 and $row.commits.0 == $row.commits.1
    }
}
";

#[test]
fn zips_two_tables() -> Result {
    let mut tester = test();
    let _: () = tester.run(ZIP_POWERED_TEST_ASSERTION_COMMAND)?;
    let code = "
        let contributors = ([
            [name, commits];
            [andres,    10]
            [    jt,    20]
        ]);

        let actual = ($contributors | upsert commits {|i| ($i.commits + 10) });
        expect $actual --to-eq [[name, commits]; [andres, 20], [jt, 30]]
    ";

    tester.run(code).expect_value_eq(true)
}

#[test]
fn zips_two_lists() -> Result {
    let code = "
        echo [0 2 4 6 8]
        | zip [1 3 5 7 9]
        | flatten
        | into string
        | str join '-'
    ";

    test().run(code).expect_value_eq("0-1-2-3-4-5-6-7-8-9")
}
