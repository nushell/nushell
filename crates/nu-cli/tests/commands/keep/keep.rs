use nu_test_support::fs::Stub::FileWithContentToBeTrimmed;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

#[test]
fn rows() {
    Playground::setup("keep_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "caballeros.csv",
            r#"
                name,lucky_code
                Andrés,1
                Jonathan,1
                Jason,2
                Yehuda,1
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                open caballeros.csv
                | keep 3
                | get lucky_code
                | math sum
                | echo $it
                "#
        ));

        assert_eq!(actual.out, "4");
    })
}
