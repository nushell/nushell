use nu_test_support::fs::Stub::FileWithContentToBeTrimmed;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

#[test]
fn table_to_tsv_text_and_from_tsv_text_back_into_table() {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        "open caco3_plastics.tsv | to tsv | from tsv | first 1 | get origin"
    );

    assert_eq!(actual.out, "SPAIN");
}

#[test]
fn table_to_tsv_text_and_from_tsv_text_back_into_table_using_csv_separator() {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        r"open caco3_plastics.tsv | to tsv | from csv --separator '\t' | first 1 | get origin"
    );

    assert_eq!(actual.out, "SPAIN");
}

#[test]
fn table_to_tsv_text() {
    Playground::setup("filter_to_tsv_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "tsv_text_sample.txt",
            r#"
                importer	shipper	tariff_item	name	origin
                Plasticos Rival	Reverte	2509000000	Calcium carbonate	Spain
                Tigre Ecuador	OMYA Andina	3824909999	Calcium carbonate	Colombia
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                open tsv_text_sample.txt
                | lines
                | split column "\t" a b c d origin
                | last 1
                | to tsv
                | lines
                | nth 1
            "#
        ));

        assert!(actual.out.contains("Colombia"));
    })
}

#[test]
fn table_to_tsv_text_skipping_headers_after_conversion() {
    Playground::setup("filter_to_tsv_test_2", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "tsv_text_sample.txt",
            r#"
                importer    shipper tariff_item name    origin
                Plasticos Rival Reverte 2509000000  Calcium carbonate   Spain
                Tigre Ecuador   OMYA Andina 3824909999  Calcium carbonate   Colombia
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                open tsv_text_sample.txt
                | lines
                | split column "\t" a b c d origin
                | last 1
                | to tsv --noheaders
            "#
        ));

        assert!(actual.out.contains("Colombia"));
    })
}

#[test]
fn from_tsv_text_to_table() {
    Playground::setup("filter_from_tsv_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "los_tres_amigos.txt",
            r#"
                first Name	Last Name	rusty_luck
                Andrés	Robalino	1
                Jonathan	Turner	1
                Yehuda	Katz	1
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                open los_tres_amigos.txt
                | from tsv
                | get rusty_luck
                | count
            "#
        ));

        assert_eq!(actual.out, "3");
    })
}

#[test]
fn from_tsv_text_skipping_headers_to_table() {
    Playground::setup("filter_from_tsv_test_2", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "los_tres_amigos.txt",
            r#"
                Andrés	Robalino	1
                Jonathan	Turner	1
                Yehuda	Katz	1
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                open los_tres_amigos.txt
                | from tsv --noheaders
                | get Column3
                | count
            "#
        ));

        assert_eq!(actual.out, "3");
    })
}
