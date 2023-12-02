use nu_test_support::{fs::Stub::FileWithContentToBeTrimmed, nu, pipeline, playground::Playground};

#[test]
fn summarizes_by_column_given() {
    Playground::setup("histogram_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "los_tres_caballeros.csv",
            r#"
                first_name,last_name,rusty_at
                Andrés,Robalino,Ecuador
                JT,Turner,Estados Unidos
                Yehuda,Katz,Estados Unidos
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                open los_tres_caballeros.csv
                | histogram rusty_at countries --percentage-type relative
                | where rusty_at == "Ecuador"
                | get countries
                | get 0
            "#
        ));

        assert_eq!(
            actual.out,
            "**************************************************"
        );
        // 50%
    })
}

#[test]
fn summarizes_by_column_given_with_normalize_percentage() {
    Playground::setup("histogram_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "los_tres_caballeros.csv",
            r#"
                first_name,last_name,rusty_at
                Andrés,Robalino,Ecuador
                JT,Turner,Estados Unidos
                Yehuda,Katz,Estados Unidos
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                open los_tres_caballeros.csv
                | histogram rusty_at countries
                | where rusty_at == "Ecuador"
                | get countries
                | get 0
            "#
        ));

        assert_eq!(actual.out, "*********************************");
        // 33%
    })
}

#[test]
fn summarizes_by_values() {
    Playground::setup("histogram_test_2", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "los_tres_caballeros.csv",
            r#"
                first_name,last_name,rusty_at
                Andrés,Robalino,Ecuador
                JT,Turner,Estados Unidos
                Yehuda,Katz,Estados Unidos
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                open los_tres_caballeros.csv
                | get rusty_at
                | histogram
                | where value == "Estados Unidos"
                | get count
                | get 0
            "#
        ));

        assert_eq!(actual.out, "2");
    })
}

#[test]
fn help() {
    let help_command = nu!("help histogram");
    let help_short = nu!("histogram -h");
    let help_long = nu!("histogram --help");

    assert_eq!(help_short.out, help_command.out);
    assert_eq!(help_long.out, help_command.out);
}

#[test]
fn count() {
    let actual = nu!(pipeline(
        "
            echo [[bit];  [1] [0] [0] [0] [0] [0] [0] [1] [1]]
            | histogram bit --percentage-type relative
            | sort-by count
            | reject frequency
            | to json
        "
    ));

    let bit_json = r#"[  {    "bit": 1,    "count": 3,    "quantile": 0.5,    "percentage": "50.00%"  },  {    "bit": 0,    "count": 6,    "quantile": 1,    "percentage": "100.00%"  }]"#;

    assert_eq!(actual.out, bit_json);
}

#[test]
fn count_with_normalize_percentage() {
    let actual = nu!(pipeline(
        "
            echo [[bit];  [1] [0] [0] [0] [0] [0] [0] [1]]
            | histogram bit --percentage-type normalize
            | sort-by count
            | reject frequency
            | to json
        "
    ));

    let bit_json = r#"[  {    "bit": 1,    "count": 2,    "quantile": 0.25,    "percentage": "25.00%"  },  {    "bit": 0,    "count": 6,    "quantile": 0.75,    "percentage": "75.00%"  }]"#;

    assert_eq!(actual.out, bit_json);
}
