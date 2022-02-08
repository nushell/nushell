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
            | str collect ", "
        "#
    ));

    assert_eq!(actual.out, "rusty_at, last_name");
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
            | str collect ", "
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
            | str collect ", "
        "#
    ));

    assert_eq!(actual.out, "last name");
}
