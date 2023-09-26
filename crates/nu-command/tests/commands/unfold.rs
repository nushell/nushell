use nu_test_support::{nu, pipeline};

#[test]
fn unfold_single_element_break() {
    let actual = nu!("unfold 1 {|x| if $x == 3 { [$x] } else { [$x, ($x + 1)] }} | to nuon");

    assert_eq!(actual.out, "[1, 2, 3]");
}

#[test]
fn unfold_single_value_break() {
    let actual = nu!("unfold 1 {|x| if $x == 3 { $x } else { [$x, ($x + 1)] }} | to nuon");

    assert_eq!(actual.out, "[1, 2, 3]");
}

#[test]
fn unfold_null_break() {
    let actual = nu!("unfold 1 {|x| if $x <= 3 { [$x, ($x + 1)] }} | to nuon");

    assert_eq!(actual.out, "[1, 2, 3]");
}

#[test]
fn unfold_invalid_return() {
    let actual = nu!("unfold 0 {|x| [a b c]}");
    assert!(actual.err.contains("Invalid block return"));
}

#[test]
fn unfold_allows_null_state() {
    let actual = nu!(pipeline(
        r#"
        unfold 0 {|x|
          if $x == null {
            ["done"]
          } else if $x < 1 {
            ["going", ($x + 1)]
          } else {
            ["stopping", null]
          }
        } | to nuon
      "#
    ));

    assert_eq!(actual.out, "[going, stopping, done]");
}

#[test]
fn unfold_allows_null_output() {
    let actual = nu!(pipeline(
        r#"
        unfold 0 {|x|
          if $x == 3 {
            "done"
          } else {
            [null, ($x + 1)]
          }
        } | to nuon
      "#
    ));

    assert_eq!(actual.out, "[null, null, null, done]");
}
