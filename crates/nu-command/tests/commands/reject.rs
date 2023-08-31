use nu_test_support::{nu, pipeline};

#[test]
fn regular_columns() {
    let actual = nu!(pipeline(
        r#"
            echo [
                [first_name, last_name, rusty_at, type];

                [Andrés Robalino '10/11/2013' A]
                [JT Turner '10/12/2013' B]
                [Yehuda Katz '10/11/2013' A]
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
    let actual = nu!("[ {a: 1, b: 2,c:txt}, { a:val } ] | reject a | get c?.0");

    assert_eq!(actual.out, "txt");
}

#[test]
fn complex_nested_columns() {
    let actual = nu!(pipeline(
        r#"
            {
                "nu": {
                    "committers": [
                        {"name": "Andrés N. Robalino"},
                        {"name": "JT Turner"},
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
    let actual = nu!(pipeline(
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
    let actual = nu!(r#"{"a": 3} | reject a | describe"#);

    assert!(actual.out.contains("record"));
}

#[test]
fn reject_table_from_raw_eval() {
    let actual = nu!(r#"[{"a": 3}] | reject a"#);

    assert!(actual.out.contains("record 0 fields"));
}

#[test]
fn reject_nested_field() {
    let actual = nu!("{a:{b:3,c:5}} | reject a.b | debug");

    assert_eq!(actual.out, "{a: {c: 5}}");
}

#[test]
fn reject_two_identical_elements() {
    let actual = nu!("[[a, a]; [1, 2]] | reject a");

    assert!(actual.out.contains("record 0 fields"));
}

#[test]
fn reject_large_vec_with_two_identical_elements() {
    let actual = nu!("[[a, b, c, d, e, a]; [1323, 23, 45, 100, 2, 2423]] | reject a");

    assert!(!actual.out.contains("1323"));
    assert!(!actual.out.contains("2423"));
    assert!(actual.out.contains('b'));
    assert!(actual.out.contains('c'));
    assert!(actual.out.contains('d'));
    assert!(actual.out.contains('e'));
    assert!(actual.out.contains("23"));
    assert!(actual.out.contains("45"));
    assert!(actual.out.contains("100"));
    assert!(actual.out.contains('2'));
}

#[test]
fn reject_optional_column() {
    let actual = nu!("{} | reject foo? | to nuon");
    assert_eq!(actual.out, "{}");

    let actual = nu!("[{}] | reject foo? | to nuon");
    assert_eq!(actual.out, "[{}]");

    let actual = nu!("[{} {foo: 2}] | reject foo? | to nuon");
    assert_eq!(actual.out, "[{}, {}]");

    let actual = nu!("[{foo: 1} {foo: 2}] | reject foo? | to nuon");
    assert_eq!(actual.out, "[{}, {}]");
}

#[test]
fn reject_optional_row() {
    let actual = nu!("[{foo: 'bar'}] | reject 3? | to nuon");
    assert_eq!(actual.out, "[[foo]; [bar]]");
}

#[test]
fn reject_multiple_rows_ascending() {
    assert_eq!(
        nu!("[[a,b];[1 2] [3 4] [5 6]] | reject 1 2 | to nuon"),
        "[[a, b]; [1, 2]]"
    )
}

#[test]
fn reject_multiple_rows_descending() {
    let actual = nu!("[[a,b];[1 2] [3 4] [5 6]] | reject 2 1");

    assert!(actual.out.contains("a"));
    assert!(actual.out.contains("b"));
    assert!(actual.out.contains("1"));
    assert!(actual.out.contains("2"));
}
