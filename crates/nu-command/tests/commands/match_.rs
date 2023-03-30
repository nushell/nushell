use nu_test_support::nu;

#[test]
fn match_for_range() {
    let actual = nu!(
        cwd: ".",
        r#"match 3 { 1..10 => { print "success" } }"#
    );
    // Make sure we don't see any of these values in the output
    // As we do not auto-print loops anymore
    assert_eq!(actual.out, "success");
}

#[test]
fn match_for_range_unmatched() {
    let actual = nu!(
        cwd: ".",
        r#"match 11 { 1..10 => { print "failure" }, _ => { print "success" }}"#
    );
    // Make sure we don't see any of these values in the output
    // As we do not auto-print loops anymore
    assert_eq!(actual.out, "success");
}

#[test]
fn match_for_record() {
    let actual = nu!(
        cwd: ".",
        r#"match {a: 11} { {a: $b} => { print $b }}"#
    );
    // Make sure we don't see any of these values in the output
    // As we do not auto-print loops anymore
    assert_eq!(actual.out, "11");
}

#[test]
fn match_for_record_shorthand() {
    let actual = nu!(
        cwd: ".",
        r#"match {a: 12} { {$a} => { print $a }}"#
    );
    // Make sure we don't see any of these values in the output
    // As we do not auto-print loops anymore
    assert_eq!(actual.out, "12");
}

#[test]
fn match_list() {
    let actual = nu!(
        cwd: ".",
        r#"match [1, 2] { [$a] => { print $"single: ($a)" }, [$b, $c] => {print $"double: ($b) ($c)"}}"#
    );
    // Make sure we don't see any of these values in the output
    // As we do not auto-print loops anymore
    assert_eq!(actual.out, "double: 1 2");
}

#[test]
fn match_list_rest_ignore() {
    let actual = nu!(
        cwd: ".",
        r#"match [1, 2] { [$a, ..] => { print $"single: ($a)" }, [$b, $c] => {print $"double: ($b) ($c)"}}"#
    );
    // Make sure we don't see any of these values in the output
    // As we do not auto-print loops anymore
    assert_eq!(actual.out, "single: 1");
}

#[test]
fn match_list_rest() {
    let actual = nu!(
        cwd: ".",
        r#"match [1, 2, 3] { [$a, ..$remainder] => { print $"single: ($a) ($remainder | math sum)" }, [$b, $c] => {print $"double: ($b) ($c)"}}"#
    );
    // Make sure we don't see any of these values in the output
    // As we do not auto-print loops anymore
    assert_eq!(actual.out, "single: 1 5");
}

#[test]
fn match_constant_1() {
    let actual = nu!(
        cwd: ".",
        r#"match 2 { 1 => { print "failure"}, 2 => { print "success" }, 3 => { print "failure" }}"#
    );
    // Make sure we don't see any of these values in the output
    // As we do not auto-print loops anymore
    assert_eq!(actual.out, "success");
}

#[test]
fn match_constant_2() {
    let actual = nu!(
        cwd: ".",
        r#"match 2.3 { 1.4 => { print "failure"}, 2.3 => { print "success" }, 3 => { print "failure" }}"#
    );
    // Make sure we don't see any of these values in the output
    // As we do not auto-print loops anymore
    assert_eq!(actual.out, "success");
}

#[test]
fn match_constant_3() {
    let actual = nu!(
        cwd: ".",
        r#"match true { false => { print "failure"}, true => { print "success" }, 3 => { print "failure" }}"#
    );
    // Make sure we don't see any of these values in the output
    // As we do not auto-print loops anymore
    assert_eq!(actual.out, "success");
}

#[test]
fn match_constant_4() {
    let actual = nu!(
        cwd: ".",
        r#"match "def" { "abc" => { print "failure"}, "def" => { print "success" }, "ghi" => { print "failure" }}"#
    );
    // Make sure we don't see any of these values in the output
    // As we do not auto-print loops anymore
    assert_eq!(actual.out, "success");
}

#[test]
fn match_constant_5() {
    let actual = nu!(
        cwd: ".",
        r#"match 2019-08-23 { 2010-01-01 => { print "failure"}, 2019-08-23 => { print "success" }, 2020-02-02 => { print "failure" }}"#
    );
    // Make sure we don't see any of these values in the output
    // As we do not auto-print loops anymore
    assert_eq!(actual.out, "success");
}

#[test]
fn match_constant_6() {
    let actual = nu!(
        cwd: ".",
        r#"match 6sec { 2sec => { print "failure"}, 6sec => { print "success" }, 1min => { print "failure" }}"#
    );
    // Make sure we don't see any of these values in the output
    // As we do not auto-print loops anymore
    assert_eq!(actual.out, "success");
}

#[test]
fn match_constant_7() {
    let actual = nu!(
        cwd: ".",
        r#"match 1kib { 1kb => { print "failure"}, 1kib => { print "success" }, 2kb => { print "failure" }}"#
    );
    // Make sure we don't see any of these values in the output
    // As we do not auto-print loops anymore
    assert_eq!(actual.out, "success");
}

#[test]
fn match_or_pattern() {
    let actual = nu!(
        cwd: ".",
        r#"match {b: 7} { {a: $a} | {b: $b} => { print $"success: ($b)" }, _ => { print "failure" }}"#
    );
    // Make sure we don't see any of these values in the output
    // As we do not auto-print loops anymore
    assert_eq!(actual.out, "success: 7");
}

#[test]
fn match_or_pattern_overlap_1() {
    let actual = nu!(
        cwd: ".",
        r#"match {a: 7} { {a: $b} | {b: $b} => { print $"success: ($b)" }, _ => { print "failure" }}"#
    );
    // Make sure we don't see any of these values in the output
    // As we do not auto-print loops anymore
    assert_eq!(actual.out, "success: 7");
}

#[test]
fn match_or_pattern_overlap_2() {
    let actual = nu!(
        cwd: ".",
        r#"match {b: 7} { {a: $b} | {b: $b} => { print $"success: ($b)" }, _ => { print "failure" }}"#
    );
    // Make sure we don't see any of these values in the output
    // As we do not auto-print loops anymore
    assert_eq!(actual.out, "success: 7");
}

#[test]
fn match_doesnt_overwrite_variable() {
    let actual = nu!(
        cwd: ".",
        r#"let b = 100; match 55 { $b => {} }; print $b"#
    );
    // Make sure we don't see any of these values in the output
    // As we do not auto-print loops anymore
    assert_eq!(actual.out, "100");
}
