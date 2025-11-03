use nu_test_support::fs::Stub::FileWithContentToBeTrimmed;
use nu_test_support::nu;
use nu_test_support::playground::Playground;

#[test]
fn table_to_tsv_text_and_from_tsv_text_back_into_table() {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        "open caco3_plastics.tsv | to tsv | from tsv | first | get origin"
    );

    assert_eq!(actual.out, "SPAIN");
}

#[test]
fn table_to_tsv_text_and_from_tsv_text_back_into_table_using_csv_separator() {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        r#"open caco3_plastics.tsv | to tsv | from csv --separator "\t" | first | get origin"#
    );

    assert_eq!(actual.out, "SPAIN");
}

#[test]
fn table_to_tsv_text() {
    Playground::setup("filter_to_tsv_test_1", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "tsv_text_sample.txt",
            r#"
                importer	shipper	tariff_item	name	origin
                Plasticos Rival	Reverte	2509000000	Calcium carbonate	Spain
                Tigre Ecuador	OMYA Andina	3824909999	Calcium carbonate	Colombia
            "#,
        )]);

        let actual = nu!(cwd: dirs.test(), r#"
            open tsv_text_sample.txt
            | lines
            | split column "\t" a b c d origin
            | last 1
            | to tsv
            | lines
            | select 1
        "#);

        assert!(actual.out.contains("Colombia"));
    })
}

#[test]
fn table_to_tsv_text_skipping_headers_after_conversion() {
    Playground::setup("filter_to_tsv_test_2", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "tsv_text_sample.txt",
            r#"
                importer    shipper tariff_item name    origin
                Plasticos Rival Reverte 2509000000  Calcium carbonate   Spain
                Tigre Ecuador   OMYA Andina 3824909999  Calcium carbonate   Colombia
            "#,
        )]);

        let actual = nu!(cwd: dirs.test(), r#"
            open tsv_text_sample.txt
            | lines
            | split column "\t" a b c d origin
            | last 1
            | to tsv --noheaders
        "#);

        assert!(actual.out.contains("Colombia"));
    })
}

#[test]
fn table_to_tsv_float_doesnt_become_int() {
    let actual = nu!(r#"
        [[a]; [1.0]] | to tsv | from tsv | get 0.a | describe
    "#);

    assert_eq!(actual.out, "float")
}

#[test]
fn from_tsv_text_to_table() {
    Playground::setup("filter_from_tsv_test_1", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "los_tres_amigos.txt",
            r#"
                first Name	Last Name	rusty_luck
                Andrés	Robalino	1
                JT	Turner	1
                Yehuda	Katz	1
            "#,
        )]);

        let actual = nu!(cwd: dirs.test(), r#"
            open los_tres_amigos.txt
            | from tsv
            | get rusty_luck
            | length
        "#);

        assert_eq!(actual.out, "3");
    })
}

#[test]
#[ignore = "csv crate has a bug when the last line is a comment: https://github.com/BurntSushi/rust-csv/issues/363"]
fn from_tsv_text_with_comments_to_table() {
    Playground::setup("filter_from_tsv_test_2", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "los_tres_caballeros.txt",
            r#"
                # This is a comment
                first_name	last_name	rusty_luck
                # This one too
                Andrés	Robalino	1
                Jonathan	Turner	1
                Yehuda	Katz	1
                # This one also
            "#,
        )]);

        let actual = nu!(cwd: dirs.test(), r##"
            open los_tres_caballeros.txt
            | from tsv --comment "#"
            | get rusty_luck
            | length
        "##);

        assert_eq!(actual.out, "3");
    })
}

#[test]
fn from_tsv_text_with_custom_quotes_to_table() {
    Playground::setup("filter_from_tsv_test_3", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "los_tres_caballeros.txt",
            r#"
                first_name	last_name	rusty_luck
                'And''rés'	Robalino	1
                Jonathan	Turner	1
                Yehuda	Katz	1
            "#,
        )]);

        let actual = nu!(cwd: dirs.test(), r#"
            open los_tres_caballeros.txt
            | from tsv --quote "'"
            | first
            | get first_name
        "#);

        assert_eq!(actual.out, "And'rés");
    })
}

#[test]
fn from_tsv_text_with_custom_escapes_to_table() {
    Playground::setup("filter_from_tsv_test_4", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "los_tres_caballeros.txt",
            r#"
                first_name	last_name	rusty_luck
                "And\"rés"	Robalino	1
                Jonathan	Turner	1
                Yehuda	Katz	1
            "#,
        )]);

        let actual = nu!(cwd: dirs.test(), r"
            open los_tres_caballeros.txt
            | from tsv --escape '\'
            | first
            | get first_name
        ");

        assert_eq!(actual.out, "And\"rés");
    })
}

#[test]
fn from_tsv_text_skipping_headers_to_table() {
    Playground::setup("filter_from_tsv_test_5", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "los_tres_amigos.txt",
            r#"
                Andrés	Robalino	1
                JT	Turner	1
                Yehuda	Katz	1
            "#,
        )]);

        let actual = nu!(cwd: dirs.test(), r#"
            open los_tres_amigos.txt
            | from tsv --noheaders
            | get column2
            | length
        "#);

        assert_eq!(actual.out, "3");
    })
}

#[test]
fn from_tsv_text_with_missing_columns_to_table() {
    Playground::setup("filter_from_tsv_test_6", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "los_tres_caballeros.txt",
            r#"
                first_name	last_name	rusty_luck
                Andrés	Robalino
                Jonathan	Turner	1
                Yehuda	Katz	1
            "#,
        )]);

        let actual = nu!(cwd: dirs.test(), r#"
            open los_tres_caballeros.txt
            | from tsv --flexible
            | get -o rusty_luck
            | compact
            | length
        "#);

        assert_eq!(actual.out, "2");
    })
}

#[test]
fn from_tsv_text_with_multiple_char_comment() {
    Playground::setup("filter_from_tsv_test_7", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "los_tres_caballeros.txt",
            r#"
                first_name	last_name	rusty_luck
                Andrés	Robalino	1
                Jonathan	Turner	1
                Yehuda	Katz	1
            "#,
        )]);

        let actual = nu!(cwd: dirs.test(), r#"
            open los_tres_caballeros.txt
            | from csv --comment "li"
        "#);

        assert!(actual.err.contains("single character separator"));
    })
}

#[test]
fn from_tsv_text_with_wrong_type_comment() {
    Playground::setup("filter_from_csv_test_8", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "los_tres_caballeros.txt",
            r#"
                first_name	last_name	rusty_luck
                Andrés	Robalino	1
                Jonathan	Turner	1
                Yehuda	Katz	1
            "#,
        )]);

        let actual = nu!(cwd: dirs.test(), r#"
            open los_tres_caballeros.txt
            | from csv --comment ('123' | into int)
        "#);

        assert!(actual.err.contains("can't convert int to char"));
    })
}
