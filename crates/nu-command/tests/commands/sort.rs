use nu_test_support::nu;

#[test]
fn by_invalid_types() {
    let actual = nu!(cwd: "tests/fixtures/formats", r#"
        open cargo_sample.toml --raw
        | echo ["foo" 1]
        | sort
        | to json -r
    "#);

    let json_output = r#"[1,"foo"]"#;
    assert_eq!(actual.out, json_output);
}

#[test]
fn sort_primitive_values() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open cargo_sample.toml --raw
        | lines
        | skip 1
        | first 6
        | sort
        | first
    ");

    assert_eq!(actual.out, "authors = [\"The Nushell Project Developers\"]");
}

#[test]
fn sort_table() {
    // if a table's records are compared directly rather than holistically as a table,
    // [100, 10, 5] will come before [100, 5, 8] because record comparison
    // compares columns by alphabetical order, so price will be checked before quantity
    let actual =
        nu!("[[id, quantity, price]; [100, 10, 5], [100, 5, 8], [100, 5, 1]] | sort | to nuon");

    assert_eq!(
        actual.out,
        r#"[[id, quantity, price]; [100, 5, 1], [100, 5, 8], [100, 10, 5]]"#
    );
}

#[test]
fn sort_different_types() {
    let actual = nu!("[a, 1, b, 2, c, 3, [4, 5, 6], d, 4, [1, 2, 3]] | sort | to json --raw");

    let json_output = r#"[1,2,3,4,"a","b","c","d",[1,2,3],[4,5,6]]"#;
    assert_eq!(actual.out, json_output);
}

#[test]
fn sort_natural() {
    let actual = nu!("['1' '2' '3' '4' '5' '10' '100'] | sort -n | to nuon");

    assert_eq!(actual.out, r#"["1", "2", "3", "4", "5", "10", "100"]"#);
}

#[test]
fn sort_record_natural() {
    let actual = nu!("{10:0,99:0,1:0,9:0,100:0} | sort -n | to nuon");

    assert_eq!(
        actual.out,
        r#"{"1": 0, "9": 0, "10": 0, "99": 0, "100": 0}"#
    );
}

#[test]
fn sort_record_insensitive() {
    let actual = nu!("{abe:1,zed:2,ABE:3} | sort -i | to nuon");

    assert_eq!(actual.out, r#"{abe: 1, ABE: 3, zed: 2}"#);
}

#[test]
fn sort_record_insensitive_reverse() {
    let actual = nu!("{abe:1,zed:2,ABE:3} | sort -ir | to nuon");

    assert_eq!(actual.out, r#"{zed: 2, ABE: 3, abe: 1}"#);
}

#[test]
fn sort_record_values_natural() {
    let actual = nu!(r#"{1:"1",2:"2",4:"100",3:"10"} | sort -vn | to nuon"#);

    assert_eq!(actual.out, r#"{"1": "1", "2": "2", "3": "10", "4": "100"}"#);
}

#[test]
fn sort_record_values_insensitive() {
    let actual = nu!("{1:abe,2:zed,3:ABE} | sort -vi | to nuon");

    assert_eq!(actual.out, r#"{"1": abe, "3": ABE, "2": zed}"#);
}

#[test]
fn sort_record_values_insensitive_reverse() {
    let actual = nu!("{1:abe,2:zed,3:ABE} | sort -vir | to nuon");

    assert_eq!(actual.out, r#"{"2": zed, "3": ABE, "1": abe}"#);
}

#[test]
fn sort_empty() {
    let actual = nu!("[] | sort | to nuon");

    assert_eq!(actual.out, "[]");
}
