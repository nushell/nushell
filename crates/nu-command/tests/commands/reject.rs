use nu_test_support::{nu, pipeline};

#[test]
fn regular_columns() {
    let actual = nu!(cwd: ".", pipeline(
        r#"
            echo [
                [first_name, last_name, rusty_at, type];

                [Andrés Robalino 10/11/2013 A]
                [Jonathan Turner 10/12/2013 B]
                [Yehuda Katz 10/11/2013 A]
            ]
            | reject type first_name
            | columns
            | str join ", "
        "#
    ));

    assert_eq!(actual.out, "last_name, rusty_at");
}

#[test]
fn skip_cell_rejection() {
    let actual = nu!(cwd: ".", pipeline(
        r#"[ {a: 1, b: 2,c:txt}, { a:val } ] | reject a | get c.0"#));

    assert_eq!(actual.out, "txt");
}

// FIXME: needs more work
#[ignore]
#[test]
fn complex_nested_columns() {
    let actual = nu!(cwd: ".", pipeline(
        r#"
            {
                "nu": {
                    "committers": [
                        {"name": "Andrés N. Robalino"},
                        {"name": "Jonathan Turner"},
                        {"name": "Yehuda Katz"}
                    ],
                    "releases": [
                        {"version": "0.2"}
                        {"version": "0.8"},
                        {"version": "0.9999999"}
                    ],
                    "0xATYKARNU": [
                        ["Th", "e", " "],
                        ["BIG", " ", "UnO"],
                        ["punto", "cero"]
                    ]
                }
            }
            | reject nu."0xATYKARNU" nu.committers
            | get nu
            | columns
            | str join ", "
        "#,
    ));

    assert_eq!(actual.out, "releases");
}

#[test]
fn ignores_duplicate_columns_rejected() {
    let actual = nu!(cwd: ".", pipeline(
        r#"
            echo [
                ["first name", "last name"];

                [Andrés Robalino]
                [Andrés Jnth]
            ]
            | reject "first name" "first name"
            | columns
            | str join ", "
        "#
    ));

    assert_eq!(actual.out, "last name");
}

#[test]
fn reject_record_from_raw_eval() {
    let actual = nu!(
        cwd: ".", pipeline(
            r#"
            {"a": 3, "a": 4} | reject a | describe
            "#
        )
    );

    assert!(actual.out.contains("record<>"));
}

#[test]
fn reject_table_from_raw_eval() {
    let actual = nu!(
        cwd: ".", pipeline(
            r#"
            [{"a": 3, "a": 4}] | reject a
            "#
        )
    );

    assert!(actual.out.contains("record 0 fields"));
}

#[test]
fn reject_nested_field() {
    let actual = nu!(
        cwd: ".", pipeline(
            r#"
            {a:{b:3,c:5}} | reject a.b | debug
            "#
        )
    );

    assert_eq!(actual.out, "{a: {c: 5}}");
}
