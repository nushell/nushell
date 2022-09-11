use nu_test_support::fs::Stub::FileWithContent;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

const ZIP_POWERED_TEST_ASSERTION_SCRIPT: &str = r#"
export def expect [
    left,
    --to-eq,
    right
] {
    $left | zip $right | all {|row|
        $row.name.0 == $row.name.1 && $row.commits.0 == $row.commits.1
    }
}
"#;

#[test]
fn zips_two_tables() {
    Playground::setup("zip_test_1", |dirs, nu| {
        nu.with_files(vec![FileWithContent(
            "zip_test.nu",
            &format!("{}\n", ZIP_POWERED_TEST_ASSERTION_SCRIPT),
        )]);

        let actual = nu!(
            cwd: ".", pipeline(
            &format!(
                r#"
                use {} expect ;

                let contributors = ([
                  [name, commits];
                  [andres,    10]
                  [    jt,    20]
                ]);

                let actual = ($contributors | upsert commits {{ |i| ($i.commits + 10) }});

                expect $actual --to-eq [[name, commits]; [andres, 20] [jt, 30]]
                "#,
                dirs.test().join("zip_test.nu").display()
            )
        ));

        assert_eq!(actual.out, "true");
    })
}

#[test]
fn zips_two_lists() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
            echo [0 2 4 6 8] | zip [1 3 5 7 9] | flatten | into string | str join '-'
        "#
    ));

    assert_eq!(actual.out, "0-1-2-3-4-5-6-7-8-9");
}
