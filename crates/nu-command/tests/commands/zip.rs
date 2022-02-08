use nu_test_support::fs::Stub::FileWithContent;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

const ZIP_POWERED_TEST_ASSERTION_SCRIPT: &str = r#"
def expect [
    left,
    right,
    --to-eq
] {
    $left | zip { $right } | all? {
        $it.name.0 == $it.name.1 && $it.commits.0 == $it.commits.1
    }
}

def add-commits [n] {
  each {
    let contributor = $it;
    let name = $it.name;
    let commits = $it.commits;

    $contributor | merge {
      [[commits]; [($commits + $n)]]
    }
  }
}
"#;

// FIXME: jt: needs more work
#[ignore]
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
                source {} ;
        
                let contributors = ([
                  [name, commits];
                  [andres,    10]
                  [    jt,    20]
                ]);
                        
                let actual = ($contributors | add-commits 10);
                        
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
                echo [0 2 4 6 8] | zip [1 3 5 7 9] | flatten | into string | str collect '-'
            "#
    ));

    assert_eq!(actual.out, "0-1-2-3-4-5-6-7-8-9");
}
