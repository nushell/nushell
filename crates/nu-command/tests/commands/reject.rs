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
fn ignores_duplicate_rows_rejected() {
    let actual = nu!("[[a,b];[1 2] [3 4] [5 6]] | reject 2 2 | to nuon");
    assert_eq!(actual.out, "[[a, b]; [1, 2], [3, 4]]");
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
fn reject_columns_with_list_spread() {
    let actual = nu!(
        "let arg = [type size]; [[name type size];[Cargo.toml file 10mb] [Cargo.lock file 10mb] [src dir 100mb]] | reject ...$arg | to nuon"
    );
    assert_eq!(
        actual.out,
        r#"[[name]; ["Cargo.toml"], ["Cargo.lock"], [src]]"#
    );
}

#[test]
fn reject_rows_with_list_spread() {
    let actual = nu!(
        "let arg = [2 0]; [[name type size];[Cargo.toml file 10mb] [Cargo.lock file 10mb] [src dir 100mb]] | reject ...$arg | to nuon"
    );
    assert_eq!(
        actual.out,
        r#"[[name, type, size]; ["Cargo.lock", file, 10000000b]]"#
    );
}

#[test]
fn reject_mixed_with_list_spread() {
    let actual = nu!(
        "let arg = [type 2]; [[name type size];[Cargp.toml file 10mb] [ Cargo.lock file 10mb] [src dir 100mb]] | reject ...$arg | to nuon"
    );
    assert_eq!(
        actual.out,
        r#"[[name, size]; ["Cargp.toml", 10000000b], ["Cargo.lock", 10000000b]]"#
    );
}

#[test]
fn reject_multiple_rows_ascending() {
    let actual = nu!("[[a,b];[1 2] [3 4] [5 6]] | reject 1 2 | to nuon");
    assert_eq!(actual.out, "[[a, b]; [1, 2]]");
}

#[test]
fn reject_multiple_rows_descending() {
    let actual = nu!("[[a,b];[1 2] [3 4] [5 6]] | reject 2 1 | to nuon");
    assert_eq!(actual.out, "[[a, b]; [1, 2]]");
}

#[test]
fn test_ignore_errors_flag() {
    let actual = nu!("[[a, b]; [1, 2], [3, 4], [5, 6]] | reject 5 -o | to nuon");
    assert_eq!(actual.out, "[[a, b]; [1, 2], [3, 4], [5, 6]]");
}

#[test]
fn test_ignore_errors_flag_var() {
    let actual =
        nu!("let arg = [5 c]; [[a, b]; [1, 2], [3, 4], [5, 6]] | reject ...$arg -o | to nuon");
    assert_eq!(actual.out, "[[a, b]; [1, 2], [3, 4], [5, 6]]");
}

#[test]
fn test_works_with_integer_path_and_stream() {
    let actual = nu!("[[N u s h e l l]] | flatten | reject 1 | to nuon");

    assert_eq!(actual.out, "[N, s, h, e, l, l]");
}
