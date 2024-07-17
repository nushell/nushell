use nu_test_support::{nu, pipeline};

#[test]
fn generate_no_next_break() {
    let actual = nu!(
        "generate {|x| if $x == 3 { {out: $x}} else { {out: $x, next: ($x + 1)} }} 1 | to nuon"
    );

    assert_eq!(actual.out, "[1, 2, 3]");
}

#[test]
fn generate_null_break() {
    let actual = nu!("generate {|x| if $x <= 3 { {out: $x, next: ($x + 1)} }} 1 | to nuon");

    assert_eq!(actual.out, "[1, 2, 3]");
}

#[test]
fn generate_allows_empty_output() {
    let actual = nu!(pipeline(
        r#"
        generate {|x|
          if $x == 1 {
            {next: ($x + 1)}
          } else if $x < 3 {
            {out: $x, next: ($x + 1)}
          }
        } 0 | to nuon
      "#
    ));

    assert_eq!(actual.out, "[0, 2]");
}

#[test]
fn generate_allows_no_output() {
    let actual = nu!(pipeline(
        r#"
        generate {|x|
          if $x < 3 {
            {next: ($x + 1)}
          }
        } 0 | to nuon
      "#
    ));

    assert_eq!(actual.out, "[]");
}

#[test]
fn generate_allows_null_state() {
    let actual = nu!(pipeline(
        r#"
        generate {|x|
          if $x == null {
            {out: "done"}
          } else if $x < 1 {
            {out: "going", next: ($x + 1)}
          } else {
            {out: "stopping", next: null}
          }
        } 0 | to nuon
      "#
    ));

    assert_eq!(actual.out, "[going, stopping, done]");
}

#[test]
fn generate_allows_null_output() {
    let actual = nu!(pipeline(
        r#"
        generate {|x|
          if $x == 3 {
            {out: "done"}
          } else {
            {out: null, next: ($x + 1)}
          }
        } 0 | to nuon
      "#
    ));

    assert_eq!(actual.out, "[null, null, null, done]");
}

#[test]
fn generate_disallows_extra_keys() {
    let actual = nu!("generate {|x| {foo: bar, out: $x}} 0 ");
    assert!(actual.err.contains("Invalid block return"));
}

#[test]
fn generate_disallows_list() {
    let actual = nu!("generate {|x| [$x, ($x + 1)]} 0 ");
    assert!(actual.err.contains("Invalid block return"));
}

#[test]
fn generate_disallows_primitive() {
    let actual = nu!("generate {|x| 1} 0");
    assert!(actual.err.contains("Invalid block return"));
}

#[test]
fn generate_allow_default_parameter() {
    let actual = nu!(pipeline(
        r#"
        generate {|x = 0|
          if $x == 3 {
            {out: "done"}
          } else {
            {out: null, next: ($x + 1)}
          }
        } | to nuon
      "#
    ));

    assert_eq!(actual.out, "[null, null, null, done]");

    // if initial is given, use initial value
    let actual = nu!(pipeline(
        r#"
        generate {|x = 0|
          if $x == 3 {
            {out: "done"}
          } else {
            {out: null, next: ($x + 1)}
          }
        } 1 | to nuon
      "#
    ));
    assert_eq!(actual.out, "[null, null, done]");
}
