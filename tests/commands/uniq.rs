use nu_test_support::fs::Stub::{FileWithContentToBeTrimmed};
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

#[test]
fn uniq_rows() {
    Playground::setup("uniq_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "los_tres_caballeros.csv",
            r#"
                first_name,last_name,rusty_at,type
                Andrés,Robalino,10/11/2013,A
                Jonathan,Turner,10/12/2013,B
                Yehuda,Katz,10/11/2013,A
                Jonathan,Turner,10/12/2013,B
                Yehuda,Katz,10/11/2013,A
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                open los_tres_caballeros.csv
                | uniq
                | count
                | echo $it
            "#
        ));

        assert_eq!(actual, "3");
    })
}

#[test]
fn uniq_columns() {
    Playground::setup("uniq_test_2", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "los_tres_caballeros.csv",
            r#"
                first_name,last_name,rusty_at,type
                Andrés,Robalino,10/11/2013,A
                Jonathan,Turner,10/12/2013,B
                Yehuda,Katz,10/11/2013,A
                Jonathan,Turner,10/12/2013,B
                Yehuda,Katz,10/11/2013,A
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                open los_tres_caballeros.csv
                | pick rusty_at type
                | uniq
                | count
                | echo $it
            "#
        ));

        assert_eq!(actual, "2");
    })
}

