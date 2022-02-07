use nu_test_support::fs::Stub::FileWithContentToBeTrimmed;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

#[test]
fn table_to_csv_text_and_from_csv_text_back_into_table() {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        "open caco3_plastics.csv | to csv | from csv | first 1 | get origin "
    );

    assert_eq!(actual.out, "SPAIN");
}

#[test]
fn table_to_csv_text() {
    Playground::setup("filter_to_csv_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "csv_text_sample.txt",
            r#"
                importer,shipper,tariff_item,name,origin
                Plasticos Rival,Reverte,2509000000,Calcium carbonate,Spain
                Tigre Ecuador,OMYA Andina,3824909999,Calcium carbonate,Colombia
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                open csv_text_sample.txt
                | lines
                | str trim
                | split column "," a b c d origin
                | last 1
                | to csv
                | lines
<<<<<<< HEAD
                | nth 1
=======
                | get 1
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
            "#
        ));

        assert!(actual
            .out
            .contains("Tigre Ecuador,OMYA Andina,3824909999,Calcium carbonate,Colombia"));
    })
}

#[test]
fn table_to_csv_text_skipping_headers_after_conversion() {
    Playground::setup("filter_to_csv_test_2", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "csv_text_sample.txt",
            r#"
                importer,shipper,tariff_item,name,origin
                Plasticos Rival,Reverte,2509000000,Calcium carbonate,Spain
                Tigre Ecuador,OMYA Andina,3824909999,Calcium carbonate,Colombia
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                open csv_text_sample.txt
                | lines
                | str trim
                | split column "," a b c d origin
                | last 1
                | to csv --noheaders
            "#
        ));

        assert!(actual
            .out
            .contains("Tigre Ecuador,OMYA Andina,3824909999,Calcium carbonate,Colombia"));
    })
}

#[test]
fn infers_types() {
    Playground::setup("filter_from_csv_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "los_cuatro_mosqueteros.csv",
            r#"
                first_name,last_name,rusty_luck,d
                Andrés,Robalino,1,d
                Jonathan,Turner,1,d
                Yehuda,Katz,1,d
                Jason,Gedge,1,d
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                open los_cuatro_mosqueteros.csv
                | where rusty_luck > 0
                | length
            "#
        ));

        assert_eq!(actual.out, "4");
    })
}

#[test]
fn from_csv_text_to_table() {
    Playground::setup("filter_from_csv_test_2", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "los_tres_caballeros.txt",
            r#"
                first_name,last_name,rusty_luck
                Andrés,Robalino,1
                Jonathan,Turner,1
                Yehuda,Katz,1
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                open los_tres_caballeros.txt
                | from csv
                | get rusty_luck
                | length
            "#
        ));

        assert_eq!(actual.out, "3");
    })
}

#[test]
fn from_csv_text_with_separator_to_table() {
    Playground::setup("filter_from_csv_test_3", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "los_tres_caballeros.txt",
            r#"
                first_name;last_name;rusty_luck
                Andrés;Robalino;1
                Jonathan;Turner;1
                Yehuda;Katz;1
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                open los_tres_caballeros.txt
<<<<<<< HEAD
                | from csv --separator ';'
=======
                | from csv --separator ";"
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
                | get rusty_luck
                | length
            "#
        ));

        assert_eq!(actual.out, "3");
    })
}

#[test]
fn from_csv_text_with_tab_separator_to_table() {
    Playground::setup("filter_from_csv_test_4", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "los_tres_caballeros.txt",
            r#"
                first_name	last_name	rusty_luck
                Andrés	Robalino	1
                Jonathan	Turner	1
                Yehuda	Katz	1
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                open los_tres_caballeros.txt
<<<<<<< HEAD
                | from csv --separator '\t'
=======
                | from csv --separator (char tab)
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
                | get rusty_luck
                | length
            "#
        ));

        assert_eq!(actual.out, "3");
    })
}

#[test]
fn from_csv_text_skipping_headers_to_table() {
    Playground::setup("filter_from_csv_test_5", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "los_tres_amigos.txt",
            r#"
                Andrés,Robalino,1
                Jonathan,Turner,1
                Yehuda,Katz,1
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                open los_tres_amigos.txt
                | from csv --noheaders
                | get Column3
                | length
            "#
        ));

        assert_eq!(actual.out, "3");
    })
}
