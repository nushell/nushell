use nu_test_support::nu;

#[test]
pub fn test_basic_non_const_success() {
    let actual = nu!("let a = 0x[00 00 00]; $a | bytes length");
    assert_eq!(
        actual.out, "3",
        "Expected the length of 0x[00 00 00] to be 3 at non-const time, but got something else"
    );
}

#[test]
pub fn test_basic_const_success() {
    let actual = nu!("const a = 0x[00 00 00] | bytes length; $a");
    assert_eq!(
        actual.out, "3",
        "Expected the length of 0x[00 00 00] to be 3 at const time, but got something else"
    );
}

#[test]
pub fn test_array_non_const_success() {
    let actual = nu!("let a = [0x[00 00 00] 0x[00]]; $a | bytes length | to nuon --raw");
    assert_eq!(
        actual.out, "[3,1]",
        "Expected the length of [0x[00 00 00], 0x[00]] to be [3, 1] at non-const time, but got something else"
    );
}

#[test]
pub fn test_array_const_success() {
    let actual = nu!("const a = [0x[00 00 00] 0x[00]] | bytes length; $a | to nuon --raw");
    assert_eq!(
        actual.out, "[3,1]",
        "Expected the length of [0x[00 00 00], 0x[00]] to be [3, 1] at const time, but got something else"
    );
}

#[test]
pub fn test_table_non_const_success() {
    let actual =
        nu!("let a = [[a]; [0x[00]] [0x[]] [0x[11 ff]]]; $a | bytes length a | to json --raw");
    assert_eq!(
        actual.out, r#"[{"a":1},{"a":0},{"a":2}]"#,
        "Failed to update table cell-paths with bytes length at non-const time"
    );
}

#[test]
pub fn test_table_const_success() {
    let actual =
        nu!("const a = [[a]; [0x[00]] [0x[]] [0x[11 ff]]] | bytes length a; $a | to json --raw");
    assert_eq!(
        actual.out, r#"[{"a":1},{"a":0},{"a":2}]"#,
        "Failed to update table cell-paths with bytes length at non-const time"
    );
}

#[test]
pub fn test_non_const_invalid_input() {
    let actual = nu!("let a = 0; $a | bytes length");
    assert!(
        actual.err.contains("command doesn't support int input"),
        "Expected error message when feeding bytes length non-bytes input at non-const time"
    );
}

#[test]
pub fn test_const_invalid_input() {
    let actual = nu!("const a = 0 | bytes length");
    assert!(
        actual.err.contains("command doesn't support int input"),
        "Expected error message when feeding bytes length non-bytes input at const time"
    );
}
