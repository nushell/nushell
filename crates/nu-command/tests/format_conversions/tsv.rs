use nu_test_support::{fs::Stub::FileWithContentToBeTrimmed, prelude::*};

#[test]
fn table_to_tsv_text_and_from_tsv_text_back_into_table() -> Result {
    let code = "open caco3_plastics.tsv | to tsv | from tsv | first | get origin";
    let outcome: String = test().cwd("tests/fixtures/formats").run(code)?;
    assert_eq!(outcome, "SPAIN");
    Ok(())
}

#[test]
fn table_to_tsv_text_and_from_tsv_text_back_into_table_using_csv_separator() -> Result {
    let code =
        r#"open caco3_plastics.tsv | to tsv | from csv --separator "\t" | first | get origin"#;
    let outcome: String = test().cwd("tests/fixtures/formats").run(code)?;
    assert_eq!(outcome, "SPAIN");
    Ok(())
}

#[test]
fn table_to_tsv_text() -> Result {
    Playground::setup("filter_to_tsv_test_1", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "tsv_text_sample.txt",
            r#"
                importer	shipper	tariff_item	name	origin
                Plasticos Rival	Reverte	2509000000	Calcium carbonate	Spain
                Tigre Ecuador	OMYA Andina	3824909999	Calcium carbonate	Colombia
            "#,
        )]);

        let code = r#"
            open tsv_text_sample.txt
            | lines
            | split column "\t" a b c d origin
            | last 1
            | to tsv
            | lines
            | get 1
        "#;

        let outcome: String = test().cwd(dirs.test()).run(code)?;
        assert_contains("Colombia", outcome);
        Ok(())
    })
}

#[test]
fn table_to_tsv_text_skipping_headers_after_conversion() -> Result {
    Playground::setup("filter_to_tsv_test_2", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "tsv_text_sample.txt",
            r#"
                importer    shipper tariff_item name    origin
                Plasticos Rival Reverte 2509000000  Calcium carbonate   Spain
                Tigre Ecuador   OMYA Andina 3824909999  Calcium carbonate   Colombia
            "#,
        )]);

        let code = r#"
            open tsv_text_sample.txt
            | lines
            | split column "\t" a b c d origin
            | last 1
            | to tsv --noheaders
        "#;

        let outcome: String = test().cwd(dirs.test()).run(code)?;
        assert_contains("Colombia", outcome);
        Ok(())
    })
}

#[test]
fn table_to_tsv_float_doesnt_become_int() -> Result {
    let code = "[[a]; [1.0]] | to tsv | from tsv | get 0.a | describe";

    let outcome: String = test().run(code)?;
    assert_eq!(outcome, "float");
    Ok(())
}

#[test]
fn from_tsv_text_to_table() -> Result {
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

        let code = r#"
            open los_tres_amigos.txt
            | from tsv
            | get rusty_luck
            | length
        "#;

        let outcome: u32 = test().cwd(dirs.test()).run(code)?;
        assert_eq!(outcome, 3);
        Ok(())
    })
}

#[test]
#[ignore = "csv crate has a bug when the last line is a comment: https://github.com/BurntSushi/rust-csv/issues/363"]
fn from_tsv_text_with_comments_to_table() -> Result {
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

        let code = r##"
            open los_tres_caballeros.txt
            | from tsv --comment "#"
            | get rusty_luck
            | length
        "##;

        let outcome: u32 = test().cwd(dirs.test()).run(code)?;
        assert_eq!(outcome, 3);
        Ok(())
    })
}

#[test]
fn from_tsv_text_with_custom_quotes_to_table() -> Result {
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

        let code = r#"
            open los_tres_caballeros.txt
            | from tsv --quote "'"
            | first
            | get first_name
        "#;

        let outcome: String = test().cwd(dirs.test()).run(code)?;
        assert_eq!(outcome, "And'rés");
        Ok(())
    })
}

#[test]
fn from_tsv_text_with_custom_escapes_to_table() -> Result {
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

        let code = r#"
            open los_tres_caballeros.txt
            | from tsv --escape '\'
            | first
            | get first_name
        "#;

        let outcome: String = test().cwd(dirs.test()).run(code)?;
        assert_eq!(outcome, "And\"rés");
        Ok(())
    })
}

#[test]
fn from_tsv_text_skipping_headers_to_table() -> Result {
    Playground::setup("filter_from_tsv_test_5", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "los_tres_amigos.txt",
            r#"
                Andrés	Robalino	1
                JT	Turner	1
                Yehuda	Katz	1
            "#,
        )]);

        let code = r#"
            open los_tres_amigos.txt
            | from tsv --noheaders
            | get column2
            | length
        "#;

        let outcome: u32 = test().cwd(dirs.test()).run(code)?;
        assert_eq!(outcome, 3);
        Ok(())
    })
}

#[test]
fn from_tsv_text_with_missing_columns_to_table() -> Result {
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

        let code = r#"
            open los_tres_caballeros.txt
            | from tsv --flexible
            | get -o rusty_luck
            | compact
            | length
        "#;

        let outcome: u32 = test().cwd(dirs.test()).run(code)?;
        assert_eq!(outcome, 2);
        Ok(())
    })
}

#[test]
fn from_tsv_text_with_multiple_char_comment() -> Result {
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

        let code = r#"
            open los_tres_caballeros.txt
            | from csv --comment "li"
        "#;

        let err = test().cwd(dirs.test()).run(code).expect_shell_error()?;
        assert_contains("single character separator", err.to_string());
        Ok(())
    })
}

#[test]
fn from_tsv_text_with_wrong_type_comment() -> Result {
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

        let code = r#"
            open los_tres_caballeros.txt
            | from csv --comment ('123' | into int)
        "#;

        let err = test().cwd(dirs.test()).run(code).expect_shell_error()?;
        let ShellError::CantConvert {
            from_type, to_type, ..
        } = err
        else {
            return Err(err.into());
        };
        assert_eq!(from_type, "int");
        assert_eq!(to_type, "char");
        Ok(())
    })
}
