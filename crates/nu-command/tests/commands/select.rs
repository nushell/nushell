use nu_test_support::{fs::Stub::EmptyFile, prelude::*};

#[test]
fn regular_columns() -> Result {
    let code = "
        [
            [first_name, last_name, rusty_at, type];
            [Andrés Robalino '10/11/2013' A]
            [JT Turner '10/12/2013' B]
            [Yehuda Katz '10/11/2013' A]
        ]
        | select rusty_at last_name
        | get 0
        | get last_name
    ";

    test().run(code).expect_value_eq("Robalino")
}

#[test]
fn complex_nested_columns() -> Result {
    let sample = json!({
        "nu": {
            "committers": [
                {"name": "Andrés N. Robalino"},
                {"name": "JT Turner"},
                {"name": "Yehuda Katz"}
            ],
            "releases": [
                {"version": "0.2"},
                {"version": "0.8"},
                {"version": "0.9999999"}
            ],
            "0xATYKARNU": [
                ["Th", "e", " "],
                ["BIG", " ", "UnO"],
                ["punto", "cero"]
            ]
        }
    });

    let code = r#"
        $in
        | select nu."0xATYKARNU" nu.committers.name nu.releases.version
        | get "nu.releases.version"
        | where $it > "0.8"
        | get 0
    "#;

    test()
        .run_with_data(code, sample)
        .expect_value_eq("0.9999999")
}

#[test]
fn fails_if_given_unknown_column_name() -> Result {
    let code = "
        [
            [first_name, last_name, rusty_at, type];
            [Andrés Robalino '10/11/2013' A]
            [JT Turner '10/12/2013' B]
            [Yehuda Katz '10/11/2013' A]
        ]
        | select rrusty_at first_name
    ";

    let err = test().run(code).expect_shell_error()?;
    // TODO: test for difference between a did-you-mean and a cant-find-column
    match err {
        ShellError::DidYouMean { suggestion, .. } => {
            assert_eq!(suggestion, "rusty_at");
            Ok(())
        }
        err => Err(err.into()),
    }
}

#[test]
fn column_names_with_spaces() -> Result {
    let code = r#"
        [
            ["first name", "last name"];
            [Andrés Robalino]
            [Andrés Jnth]
        ]
        | select "last name"
        | get "last name"
    "#;

    test().run(code).expect_value_eq(["Robalino", "Jnth"])
}

#[test]
fn ignores_duplicate_columns_selected() -> Result {
    let code = r#"
        echo [
            ["first name", "last name"];
            [Andrés Robalino]
            [Andrés Jnth]
        ]
        | select "first name" "last name" "first name"
        | columns
    "#;

    test()
        .run(code)
        .expect_value_eq(["first name", "last name"])
}

#[test]
fn selects_a_row() -> Result {
    Playground::setup("selects_a_row", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("notes.txt"), EmptyFile("arepas.txt")]);

        let code = "
            ls
            | sort-by name
            | select 0
            | get name.0
        ";

        test()
            .cwd(dirs.test())
            .run(code)
            .expect_value_eq("arepas.txt")
    })
}

#[test]
fn selects_large_row_number() -> Result {
    test()
        .run("seq 1 5 | select 9999999999 | to nuon")
        .expect_value_eq("[]")
}

#[test]
fn selects_many_rows() -> Result {
    Playground::setup("select_test_2", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("notes.txt"), EmptyFile("arepas.txt")]);

        let code = "
            ls
            | get name
            | select 1 0
            | length
        ";

        test().cwd(dirs.test()).run(code).expect_value_eq(2)
    })
}

#[test]
fn select_ignores_errors_successfully1() -> Result {
    let input = json!([
        {"a": 1, "b": 2},
        {"a": 3, "b": 5},
        {"a": 3}
    ]);

    test()
        .run_with_data("$in | select b? | length", input)
        .expect_value_eq(3)
}

#[test]
fn select_ignores_errors_successfully2() -> Result {
    test()
        .run("[{a: 1} {a: 2} {a: 3}] | select b? | to nuon")
        .expect_value_eq("[[b]; [null], [null], [null]]")
}

#[test]
fn select_ignores_errors_successfully3() -> Result {
    test()
        .run("{foo: bar} | select invalid_key? | to nuon")
        .expect_value_eq("{invalid_key: null}")
}

#[test]
fn select_ignores_errors_successfully4() -> Result {
    let input = "
        key val
        a 1
        b 2
    ";

    let code = r#"
        $in
        | lines
        | split column --collapse-empty " "
        | select foo?
        | to nuon
    "#;

    test()
        .run_with_data(code, input.trim())
        .expect_value_eq("[[foo]; [null], [null], [null]]")
}

#[test]
fn select_failed1() -> Result {
    let input = json!([
        {"a": 1, "b": 2},
        {"a": 3, "b": 5},
        {"a": 3}
    ]);

    let err = test()
        .run_with_data("$in | select b", input)
        .expect_shell_error()?;

    match err {
        ShellError::CantFindColumn { col_name, .. } => {
            assert_eq!(col_name, "b");
            Ok(())
        }
        err => Err(err.into()),
    }
}

#[test]
fn select_failed2() -> Result {
    let err = test()
        .run("[{a: 1} {a: 2} {a: 3}] | select b")
        .expect_shell_error()?;

    match err {
        ShellError::CantFindColumn { col_name, .. } => {
            assert_eq!(col_name, "b");
            Ok(())
        }
        err => Err(err.into()),
    }
}

#[test]
fn select_failed3() -> Result {
    let input = "
        key val
        a 1
        b 2
    ";

    let code = r#"
        $in
        | lines
        | split column --collapse-empty " "
        | select "100"
    "#;

    let err = test().run_with_data(code, input).expect_shell_error()?;

    match err {
        ShellError::CantFindColumn { col_name, .. } => {
            assert_eq!(col_name, "100");
            Ok(())
        }
        err => Err(err.into()),
    }
}

#[test]
fn select_repeated_rows() -> Result {
    test()
        .run("[[a b c]; [1 2 3] [4 5 6] [7 8 9]] | select 0 0 | to nuon")
        .expect_value_eq("[[a, b, c]; [1, 2, 3]]")
}

#[test]
fn select_repeated_column() -> Result {
    test()
        .run("[[a b c]; [1 2 3] [4 5 6] [7 8 9]] | select a a | to nuon")
        .expect_value_eq("[[a]; [1], [4], [7]]")
}

#[test]
fn ignore_errors_works() -> Result {
    let code = r#"
        let path = "foo";
        [{}] | select -o $path | to nuon
    "#;

    test().run(code).expect_value_eq("[[foo]; [null]]")
}

#[test]
fn select_on_empty_list_returns_empty_list() -> Result {
    test()
        .run("[] | select foo | to nuon")
        .expect_value_eq("[]")?;

    test()
        .run("[] | each {|i| $i} | select foo | to nuon")
        .expect_value_eq("[]")
}

#[test]
fn select_columns_with_list_spread() -> Result {
    let code = "
        let columns = [a c];
        echo [[a b c]; [1 2 3]] | select ...$columns | to nuon
    ";

    test().run(code).expect_value_eq("[[a, c]; [1, 3]]")
}

#[test]
fn select_rows_with_list_spread() -> Result {
    let code = "
        let rows = [0 2];
        echo [[a b c]; [1 2 3] [4 5 6] [7 8 9]] | select ...$rows | to nuon
    ";

    test()
        .run(code)
        .expect_value_eq("[[a, b, c]; [1, 2, 3], [7, 8, 9]]")
}

#[test]
fn select_single_row_with_variable() -> Result {
    test()
        .run("let idx = 2; [{a: 1, b: 2} {a: 3, b: 5} {a: 3}] | select $idx | to nuon")
        .expect_value_eq("[[a]; [3]]")
}

#[test]
fn select_with_negative_number_errors_out() -> Result {
    let err = test().run("[1 2 3] | select (-2)").expect_shell_error()?;

    match err {
        ShellError::CantConvert {
            to_type, from_type, ..
        } => {
            assert_eq!(to_type, "cell path");
            assert_eq!(from_type, "negative number");
            Ok(())
        }
        err => Err(err.into()),
    }
}
