use nu_test_support::{fs::Stub::FileWithContentToBeTrimmed, nu, pipeline, playground::Playground};

#[test]
fn condition_is_met() {
    Playground::setup("skip_while_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "caballeros.txt",
            r#"
                CHICKEN SUMMARY                        report date: April 29th, 2020
                --------------------------------------------------------------------
                Chicken Collection,29/04/2020,30/04/2020,31/04/2020
                Yellow Chickens,,,
                Andrés,0,0,1
                JT,0,0,1
                Jason,0,0,1
                Yehuda,0,0,1
                Blue Chickens,,,
                Andrés,0,0,1
                JT,0,0,1
                Jason,0,0,1
                Yehuda,0,0,2
                Red Chickens,,,
                Andrés,0,0,1
                JT,0,0,1
                Jason,0,0,1
                Yehuda,0,0,3
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
                | skip while {|row| $row."Chicken Collection" != "Red Chickens" }
                | skip 1
                | into int "31/04/2020"
                | get "31/04/2020"
                | math sum
                "#
        ));

        assert_eq!(actual.out, "6");
    })
}

#[test]
fn fail_on_non_iterator() {
    let actual = nu!("1 | skip while {|row| $row == 2}");

    assert!(actual.err.contains("command doesn't support"));
}
