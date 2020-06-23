use nu_test_support::fs::Stub::FileWithContentToBeTrimmed;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

#[test]
fn row() {
    Playground::setup("merge_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![
            FileWithContentToBeTrimmed(
                "caballeros.csv",
                r#"
                name,country,luck
                Andrés,Ecuador,0
                Jonathan,USA,0
                Jason,Canada,0
                Yehuda,USA,0
            "#,
            ),
            FileWithContentToBeTrimmed(
                "new_caballeros.csv",
                r#"
                name,country,luck
                Andrés Robalino,Guayaquil Ecuador,1
                Jonathan Turner,New Zealand,1
            "#,
            ),
        ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                open caballeros.csv
                | merge { open new_caballeros.csv }
                | where country in: ["Guayaquil Ecuador" "New Zealand"]
                | get luck
                | math sum
                | echo $it
                "#
        ));

        assert_eq!(actual.out, "2");
    })
}
