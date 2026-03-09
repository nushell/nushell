use nu_test_support::{fs::Stub::FileWithContentToBeTrimmed, prelude::*};

#[test]
fn table_to_csv_text_and_from_csv_text_back_into_table() -> Result {
    let code = "open caco3_plastics.csv | to csv | from csv | first | get origin";
    test()
        .cwd("tests/fixtures/formats")
        .run(code)
        .expect_value_eq("SPAIN")
}

#[test]
fn table_to_csv_text() -> Result {
    Playground::setup("filter_to_csv_test_1", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "csv_text_sample.txt",
            r#"
                importer,shipper,tariff_item,name,origin
                Plasticos Rival,Reverte,2509000000,Calcium carbonate,Spain
                Tigre Ecuador,OMYA Andina,3824909999,Calcium carbonate,Colombia
            "#,
        )]);

        let code = r#"
            open csv_text_sample.txt
            | lines
            | str trim
            | split column "," a b c d origin
            | last 1
            | to csv
            | lines
            | get 1
        "#;

        test()
            .cwd(dirs.test())
            .run(code)
            .expect_value_eq("Tigre Ecuador,OMYA Andina,3824909999,Calcium carbonate,Colombia")
    })
}

#[test]
fn table_to_csv_text_skipping_headers_after_conversion() -> Result {
    Playground::setup("filter_to_csv_test_2", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "csv_text_sample.txt",
            r#"
                importer,shipper,tariff_item,name,origin
                Plasticos Rival,Reverte,2509000000,Calcium carbonate,Spain
                Tigre Ecuador,OMYA Andina,3824909999,Calcium carbonate,Colombia
            "#,
        )]);

        let code = r#"
            open csv_text_sample.txt
            | lines
            | str trim
            | split column "," a b c d origin
            | last 1
            | to csv --noheaders
        "#;

        test()
            .cwd(dirs.test())
            .run(code)
            .expect_value_eq("Tigre Ecuador,OMYA Andina,3824909999,Calcium carbonate,Colombia\n")
    })
}

#[test]
fn table_to_csv_float_doesnt_become_int() -> Result {
    let code = "[[a]; [1.0]] | to csv | from csv | get 0.a";
    let outcome: Value = test().run(code)?;
    assert!(matches!(outcome, Value::Float { .. }));
    Ok(())
}

#[test]
fn infers_types() -> Result {
    Playground::setup("filter_from_csv_test_1", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "los_cuatro_mosqueteros.csv",
            r#"
                first_name,last_name,rusty_luck,d
                Andrés,Robalino,1,d
                JT,Turner,1,d
                Yehuda,Katz,1,d
                Jason,Gedge,1,d
            "#,
        )]);

        let code = r#"
            open los_cuatro_mosqueteros.csv
            | where rusty_luck > 0
            | length
        "#;

        test().cwd(dirs.test()).run(code).expect_value_eq(4)
    })
}

#[test]
fn from_csv_text_to_table() -> Result {
    Playground::setup("filter_from_csv_test_2", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "los_tres_caballeros.txt",
            r#"
                first_name,last_name,rusty_luck
                Andrés,Robalino,1
                JT,Turner,1
                Yehuda,Katz,1
            "#,
        )]);

        let code = r#"
            open los_tres_caballeros.txt
            | from csv
            | get rusty_luck
            | length
        "#;

        test().cwd(dirs.test()).run(code).expect_value_eq(3)
    })
}

#[test]
fn from_csv_text_with_separator_to_table() -> Result {
    Playground::setup("filter_from_csv_test_3", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "los_tres_caballeros.txt",
            r#"
                first_name;last_name;rusty_luck
                Andrés;Robalino;1
                JT;Turner;1
                Yehuda;Katz;1
            "#,
        )]);

        let code = r#"
            open los_tres_caballeros.txt
            | from csv --separator ";"
            | get rusty_luck
            | length
        "#;

        test().cwd(dirs.test()).run(code).expect_value_eq(3)
    })
}

#[test]
fn from_csv_text_with_tab_separator_to_table() -> Result {
    Playground::setup("filter_from_csv_test_4", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "los_tres_caballeros.txt",
            r#"
                first_name	last_name	rusty_luck
                Andrés	Robalino	1
                JT	Turner	1
                Yehuda	Katz	1
            "#,
        )]);

        let code = r#"
            open los_tres_caballeros.txt
            | from csv --separator (char tab)
            | get rusty_luck
            | length
        "#;

        test().cwd(dirs.test()).run(code).expect_value_eq(3)
    })
}

#[test]
#[ignore = "csv crate has a bug when the last line is a comment: https://github.com/BurntSushi/rust-csv/issues/363"]
fn from_csv_text_with_comments_to_table() -> Result {
    Playground::setup("filter_from_csv_test_5", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "los_tres_caballeros.txt",
            r#"
                # This is a comment
                first_name,last_name,rusty_luck
                # This one too
                Andrés,Robalino,1
                Jonathan,Turner,1
                Yehuda,Katz,1
                # This one also
            "#,
        )]);

        let code = r##"
            open los_tres_caballeros.txt
            | from csv --comment "#"
            | get rusty_luck
            | length
        "##;

        test().cwd(dirs.test()).run(code).expect_value_eq(3)
    })
}

#[test]
fn from_csv_text_with_custom_quotes_to_table() -> Result {
    Playground::setup("filter_from_csv_test_6", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "los_tres_caballeros.txt",
            r#"
                first_name,last_name,rusty_luck
                'And''rés',Robalino,1
                Jonathan,Turner,1
                Yehuda,Katz,1
            "#,
        )]);

        let code = r#"
            open los_tres_caballeros.txt
            | from csv --quote "'"
            | first
            | get first_name
        "#;

        test().cwd(dirs.test()).run(code).expect_value_eq("And'rés")
    })
}

#[test]
fn from_csv_text_with_custom_escapes_to_table() -> Result {
    Playground::setup("filter_from_csv_test_7", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "los_tres_caballeros.txt",
            r#"
                first_name,last_name,rusty_luck
                "And\"rés",Robalino,1
                Jonathan,Turner,1
                Yehuda,Katz,1
            "#,
        )]);

        let code = r#"
            open los_tres_caballeros.txt
            | from csv --escape '\'
            | first
            | get first_name
        "#;

        test()
            .cwd(dirs.test())
            .run(code)
            .expect_value_eq(r#"And"rés"#)
    })
}

#[test]
fn from_csv_text_skipping_headers_to_table() -> Result {
    Playground::setup("filter_from_csv_test_8", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "los_tres_amigos.txt",
            r#"
                Andrés,Robalino,1
                JT,Turner,1
                Yehuda,Katz,1
            "#,
        )]);

        let code = r#"
            open los_tres_amigos.txt
            | from csv --noheaders
            | get column2
            | length
        "#;

        test().cwd(dirs.test()).run(code).expect_value_eq(3)
    })
}

#[test]
fn from_csv_text_with_missing_columns_to_table() -> Result {
    Playground::setup("filter_from_csv_test_9", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "los_tres_caballeros.txt",
            r#"
                first_name,last_name,rusty_luck
                Andrés,Robalino
                Jonathan,Turner,1
                Yehuda,Katz,1
            "#,
        )]);

        let code = r#"
            open los_tres_caballeros.txt
            | from csv --flexible
            | get -o rusty_luck
            | compact
            | length
        "#;

        test().cwd(dirs.test()).run(code).expect_value_eq(2)
    })
}

#[test]
fn from_csv_text_with_multiple_char_separator() -> Result {
    Playground::setup("filter_from_csv_test_10", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "los_tres_caballeros.txt",
            r#"
                first_name,last_name,rusty_luck
                Andrés,Robalino,1
                Jonathan,Turner,1
                Yehuda,Katz,1
            "#,
        )]);

        let code = r#"
            open los_tres_caballeros.txt
            | from csv --separator "li"
        "#;

        let outcome = test().cwd(dirs.test()).run(code).expect_error()?;
        match outcome {
            ShellError::NonUtf8Custom { msg, .. } => {
                assert_eq!(msg, "separator should be a single char or a 4-byte unicode");
                Ok(())
            }
            err => Err(err.into()),
        }
    })
}

#[test]
fn from_csv_text_with_wrong_type_separator() -> Result {
    Playground::setup("filter_from_csv_test_11", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "los_tres_caballeros.txt",
            r#"
                first_name,last_name,rusty_luck
                Andrés,Robalino,1
                Jonathan,Turner,1
                Yehuda,Katz,1
            "#,
        )]);

        let code = r#"
            open los_tres_caballeros.txt
            | from csv --separator ('123' | into int)
        "#;

        let outcome = test().cwd(dirs.test()).run(code).expect_error()?;
        match outcome {
            ShellError::CantConvert {
                to_type, from_type, ..
            } => {
                assert_eq!(from_type, "int");
                assert_eq!(to_type, "string");
                Ok(())
            }
            err => Err(err.into()),
        }
    })
}

#[test]
fn table_with_record_error() -> Result {
    let code = r#"
        [[a b]; [1 2] [3 {a: 1 b: 2}]]
        | to csv
    "#;

    let outcome = test().run(code).expect_error()?;
    assert!(matches!(outcome, ShellError::CantConvert { .. }));
    Ok(())
}

#[test]
fn list_not_table_parse_time_error() -> Result {
    let code = r#"
        [{a: 1 b: 2} {a: 3 b: 4} 1]
        | to csv
    "#;

    let outcome = test().run(code).expect_parse_error()?;
    assert!(matches!(outcome, ParseError::InputMismatch { .. }));
    Ok(())
}

#[test]
fn list_not_table_runtime_error() -> Result {
    let code = r#"
        echo [{a: 1 b: 2} {a: 3 b: 4} 1]
        | to csv
    "#;

    let outcome = test().run(code).expect_shell_error()?;
    assert!(matches!(
        outcome,
        ShellError::OnlySupportsThisInputType { .. }
    ));
    Ok(())
}

#[test]
fn string_to_csv_error() -> Result {
    let code = r#"
        'qwe' | to csv
    "#;

    let outcome = test().run(code).expect_parse_error()?;
    assert!(matches!(outcome, ParseError::InputMismatch(..)));
    Ok(())
}

#[test]
fn parses_csv_with_unicode_sep() -> Result {
    Playground::setup("filter_from_csv_unicode_sep_test_3", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "los_tres_caballeros.txt",
            r#"
                first_name;last_name;rusty_luck
                Andrés;Robalino;1
                JT;Turner;1
                Yehuda;Katz;1
            "#,
        )]);

        let code = r#"
            open los_tres_caballeros.txt
            | from csv --separator "003B"
            | get rusty_luck
            | length
        "#;

        test().cwd(dirs.test()).run(code).expect_value_eq(3)
    })
}

#[test]
fn parses_csv_with_unicode_x1f_sep() -> Result {
    Playground::setup("filter_from_csv_unicode_sep_x1f_test_3", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "los_tres_caballeros.txt",
            r#"
                first_namelast_namerusty_luck
                AndrésRobalino1
                JTTurner1
                YehudaKatz1
            "#,
        )]);

        let code = r#"
            open los_tres_caballeros.txt
            | from csv --separator "001F"
            | get rusty_luck
            | length
        "#;

        test().cwd(dirs.test()).run(code).expect_value_eq(3)
    })
}

#[test]
fn from_csv_test_flexible_extra_vals() -> Result {
    let code = r#"
      echo "a,b\n1,2,3" | from csv --flexible | first | values | to nuon
    "#;

    test().run(code).expect_value_eq("[1, 2, 3]")
}

#[test]
fn from_csv_test_flexible_missing_vals() -> Result {
    let code = r#"
      echo "a,b\n1" | from csv --flexible | first | values | to nuon
    "#;

    test().run(code).expect_value_eq("[1]")
}
