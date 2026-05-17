use nu_test_support::prelude::*;

#[test]
fn by_invalid_types() -> Result {
    let code = r#"
        open cargo_sample.toml --raw
        | echo ["foo" 1]
        | sort
    "#;

    test()
        .cwd("tests/fixtures/formats")
        .run(code)
        .expect_value_eq((1, "foo"))
}

#[test]
fn sort_primitive_values() -> Result {
    let code = "
        open cargo_sample.toml --raw
        | lines
        | skip 1
        | first 6
        | sort
        | first
    ";

    test()
        .cwd("tests/fixtures/formats")
        .run(code)
        .expect_value_eq(r#"authors = ["The Nushell Project Developers"]"#)
}

#[test]
fn sort_table() -> Result {
    // if a table's records are compared directly rather than holistically as a table,
    // [100, 10, 5] will come before [100, 5, 8] because record comparison
    // compares columns by alphabetical order, so price will be checked before quantity
    let code = "[[id, quantity, price]; [100, 10, 5], [100, 5, 8], [100, 5, 1]] | sort | to nuon";

    test()
        .run(code)
        .expect_value_eq("[[id, quantity, price]; [100, 5, 1], [100, 5, 8], [100, 10, 5]]")
}

#[test]
fn sort_different_types() -> Result {
    let code = "[a, 1, b, 2, c, 3, [4, 5, 6], d, 4, [1, 2, 3]] | sort | to json --raw";

    let json_output = r#"[1,2,3,4,"a","b","c","d",[1,2,3],[4,5,6]]"#;
    test().run(code).expect_value_eq(json_output)
}

#[test]
fn sort_natural() -> Result {
    let code = "['1' '2' '3' '4' '5' '10' '100'] | sort -n";

    test()
        .run(code)
        .expect_value_eq(["1", "2", "3", "4", "5", "10", "100"])
}

#[test]
fn sort_record_natural() -> Result {
    let code = "{10:0,99:0,1:0,9:0,100:0} | sort -n | to nuon";

    test()
        .run(code)
        .expect_value_eq(r#"{"1": 0, "9": 0, "10": 0, "99": 0, "100": 0}"#)
}

#[test]
fn sort_record_insensitive() -> Result {
    let code = "{abe:1,zed:2,ABE:3} | sort -i | to nuon";

    test().run(code).expect_value_eq("{abe: 1, ABE: 3, zed: 2}")
}

#[test]
fn sort_record_insensitive_reverse() -> Result {
    let code = "{abe:1,zed:2,ABE:3} | sort -ir | to nuon";

    test().run(code).expect_value_eq("{zed: 2, ABE: 3, abe: 1}")
}

#[test]
fn sort_record_values_natural() -> Result {
    let code = r#"{1:"1",2:"2",4:"100",3:"10"} | sort -vn | to nuon"#;

    test()
        .run(code)
        .expect_value_eq(r#"{"1": "1", "2": "2", "3": "10", "4": "100"}"#)
}

#[test]
fn sort_record_values_insensitive() -> Result {
    let code = "{1:abe,2:zed,3:ABE} | sort -vi | to nuon";

    test()
        .run(code)
        .expect_value_eq(r#"{"1": abe, "3": ABE, "2": zed}"#)
}

#[test]
fn sort_record_values_insensitive_reverse() -> Result {
    let code = "{1:abe,2:zed,3:ABE} | sort -vir | to nuon";

    test()
        .run(code)
        .expect_value_eq(r#"{"2": zed, "3": ABE, "1": abe}"#)
}

#[test]
fn sort_empty() -> Result {
    let code = "[] | sort | to nuon";

    test().run(code).expect_value_eq("[]")
}
