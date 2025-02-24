use nu_test_support::{nu, pipeline};

#[test]
fn rank_primitive_values() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        "
            open cargo_sample.toml --raw
            | lines
            | skip 1
            | first 6
            | rank
            | to nuon
        "
    ));

    assert_eq!(actual.out, r#"[5.0, 6.0, 1.0, 2.0, 4.0, 3.0]"#);
}

#[test]
fn rank_table_records() {
    // if a table's records are compared directly rather than holistically as a table,
    // [100, 10, 5] will come before [100, 5, 8] because record comparison
    // compares columns by alphabetical order, so price will be checked before quantity
    let actual =
        nu!("[[id, quantity, price]; [100, 10, 5], [100, 5, 8], [100, 5, 1]] | rank | to nuon");

    assert_eq!(actual.out, r#"[3.0, 2.0, 1.0]"#);
}

#[test]
fn rank_different_types() {
    // mixed types are sorted by their enum value in `Value`
    let actual = nu!("[a, 1, b, 2, c, 3, [4, 5, 6], d, 4, [1, 2, 3]] | rank | to nuon");

    let json_output = r#"[5.0, 1.0, 6.0, 2.0, 7.0, 3.0, 10.0, 8.0, 4.0, 9.0]"#;
    assert_eq!(actual.out, json_output);
}

#[test]
fn rank_natural() {
    let actual = nu!("['1' '2' '3' '4' '5' '10' '100'] | rank -n | to nuon");

    assert_eq!(actual.out, r#"[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0]"#);
}

#[test]
fn sort_record_natural() {
    let actual = nu!("{10:0,99:0,1:0,9:0,100:0} | rank -n | to nuon");

    assert_eq!(actual.out, r#"[3.0, 4.0, 1.0, 2.0, 5.0]"#);
}

#[test]
fn rank_record_insensitive() {
    let actual = nu!("{abe:1,zed:2,ABE:3} | rank -i | to nuon");

    assert_eq!(actual.out, r#"[1.5, 3.0, 1.5]"#);
}

#[test]
fn rank_record_insensitive_reverse_dense() {
    let actual = nu!("{abe:1,zed:2,ABE:3} | rank -ir --method dense | to nuon");

    assert_eq!(actual.out, r#"[2.0, 1.0, 2.0]"#);
}

#[test]
fn rank_record_insensitive_min() {
    let actual = nu!("{abe:1,zed:2,ABE:3} | rank -i --method min | to nuon");

    assert_eq!(actual.out, r#"[1.0, 3.0, 1.0]"#);
}

#[test]
fn rank_record_insensitive_reverse_max() {
    let actual = nu!("{abe:1,zed:2,ABE:3} | rank -ir --method max | to nuon");

    assert_eq!(actual.out, r#"[3.0, 1.0, 3.0]"#);
}

#[test]
fn sort_record_values_natural() {
    let actual = nu!(r#"{1:"1",2:"2",4:"100",3:"10"} | rank -vn | to nuon"#);

    assert_eq!(actual.out, r#"[1.0, 2.0, 4.0, 3.0]"#);
}

#[test]
fn sort_record_values_insensitive() {
    let actual = nu!("{1:abe,2:zed,3:ABE} | rank -vi | to nuon");

    assert_eq!(actual.out, r#"[1.5, 3.0, 1.5]"#);
}

#[test]
fn sort_record_values_insensitive_reverse() {
    let actual = nu!("{1:abe,2:zed,3:ABE} | rank -vir | to nuon");

    assert_eq!(actual.out, r#"[2.5, 1.0, 2.5]"#);
}

#[test]
fn sort_empty() {
    let actual = nu!("[] | sort | to nuon");

    assert_eq!(actual.out, "[]");
}
