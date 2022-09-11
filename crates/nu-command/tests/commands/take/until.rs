use nu_test_support::fs::Stub::FileWithContentToBeTrimmed;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

#[test]
fn condition_is_met() {
    Playground::setup("take_until_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "caballeros.txt",
            r#"
                CHICKEN SUMMARY                        report date: April 29th, 2020
                --------------------------------------------------------------------
                Chicken Collection,29/04/2020,30/04/2020,31/04/2020
                Yellow Chickens,,,
                Andrés,1,1,1
                Jonathan,1,1,1
                Jason,1,1,1
                Yehuda,1,1,1
                Blue Chickens,,,
                Andrés,1,1,2
                Jonathan,1,1,2
                Jason,1,1,2
                Yehuda,1,1,2
                Red Chickens,,,
                Andrés,1,1,3
                Jonathan,1,1,3
                Jason,1,1,3
                Yehuda,1,1,3
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                open --raw caballeros.txt
                | lines
                | skip 2
                | str trim
                | str join (char nl)
                | from csv
                | skip while "Chicken Collection" != "Blue Chickens"
                | take until "Chicken Collection" == "Red Chickens"
                | skip 1
                | into int "31/04/2020"
                | get "31/04/2020"
                | math sum
                "#
        ));

        assert_eq!(actual.out, "8");
    })
}
