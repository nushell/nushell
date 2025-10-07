use nu_test_support::nu;

#[test]
fn removes_duplicate_rows() {
    let sample = r#"[
        [first_name , last_name, rusty_at, type];
        [Andrés     , Robalino, "10/11/2013", A],
        [Afonso     , Turner, "10/12/2013", B],
        [Yehuda     , Katz, "10/11/2013", A],
        [JT         , Turner, "11/12/2011", O]
    ]"#;

    let actual = nu!(format!("{sample} | uniq-by last_name | length"));

    assert_eq!(actual.out, "3");
}

#[test]
fn uniq_when_keys_out_of_order() {
    let actual = nu!(r#"[{"a": "a", "b": [1,2,3]}, {"b": [1,2,3,4], "a": "a"}] | uniq-by a"#);
    let expected = nu!(r#"[{"a": "a", "b": [1,2,3]}]"#);

    assert_eq!(actual.out, expected.out);
}

#[test]
fn uniq_counting() {
    let actual = nu!(r#"
        ["A", "B", "A"]
        | wrap item
        | uniq-by item --count
        | flatten
        | where item == A
        | get count
        | get 0
    "#);
    assert_eq!(actual.out, "2");

    let actual = nu!(r#"
        ["A", "B", "A"]
        | wrap item
        | uniq-by item --count
        | flatten
        | where item == B
        | get count
        | get 0
    "#);
    assert_eq!(actual.out, "1");
}

#[test]
fn uniq_unique() {
    let actual = nu!("
        echo [1 2 3 4 1 5]
        | wrap item
        | uniq-by item --unique
        | get item
    ");
    let expected = nu!("[2 3 4 5]");
    assert_eq!(actual.out, expected.out);
}

#[test]
fn table() {
    let actual = nu!("
        [[fruit day]; [apple monday] [apple friday] [Apple friday] [apple monday] [pear monday] [orange tuesday]]
        | uniq-by fruit
    ");

    let expected = nu!("
    echo [[fruit day]; [apple monday] [Apple friday] [pear monday] [orange tuesday]]
    ");
    print!("{}", actual.out);
    print!("{}", expected.out);
    assert_eq!(actual.out, expected.out);
}

#[test]
fn uniq_by_empty() {
    let actual = nu!("[] | uniq-by foo | to nuon");

    assert_eq!(actual.out, "[]");
}

#[test]
fn uniq_by_multiple_columns() {
    let actual = nu!("
        [[fruit day]; [apple monday] [apple friday] [Apple friday] [apple monday] [pear monday] [orange tuesday]]
        | uniq-by fruit day
    ");

    let expected = nu!("
    echo [[fruit day]; [apple monday] [apple friday] [Apple friday] [pear monday] [orange tuesday]]
    ");
    assert_eq!(actual.out, expected.out);
}

#[test]
fn table_with_ignore_case() {
    let actual = nu!(r#"
        [[origin, people];
            [World, (
                [[name, meal];
                    ['Geremias', {plate: 'bitoque', carbs: 100}]
                ]
            )],
            [World, (
                [[name, meal];
                    ['Martin', {plate: 'bitoque', carbs: 100}]
                ]
            )],
            [World, (
                [[name, meal];
                    ['Geremias', {plate: 'Bitoque', carbs: 100}]
                ]
            )],
        ] | uniq-by people -i
    "#);

    let expected = nu!(r#"
    echo [[origin, people];
            [World, (
                [[name, meal];
                    ['Geremias', {plate: 'bitoque', carbs: 100}]
                ]
            )],
            [World, (
                [[name, meal];
                    ['Martin', {plate: 'bitoque', carbs: 100}]
                ]
            )],
        ]
    "#);

    assert_eq!(actual.out, expected.out);
}

#[test]
fn missing_parameter() {
    let actual = nu!("[11 22 33] | uniq-by");

    assert!(actual.err.contains("missing parameter"));
}

#[test]
fn wrong_column() {
    let actual = nu!("[[fruit day]; [apple monday] [apple friday]] | uniq-by column1");

    assert!(actual.err.contains("cannot find column 'column1'"));
}
