use nu_test_support::{nu, pipeline};

#[test]
fn reports_emptiness() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
            [[] '' {} null]
            | all {
              is-empty
            }
        "#
    ));

    assert_eq!(actual.out, "true");
}

#[test]
fn reports_nonemptiness() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
            [[1] ' ' {a:1} 0]
            | any {
              is-empty
            }
        "#
    ));

    assert_eq!(actual.out, "false");
}

#[test]
fn reports_emptiness_by_columns() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
            [{a:1 b:null c:null} {a:2 b:null c:null}]
            | any {
              is-empty b c
            }
        "#
    ));

    assert_eq!(actual.out, "true");
}

#[test]
fn reports_nonemptiness_by_columns() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
            [{a:1 b:null c:3} {a:null b:5 c:2}]
            | any {
              is-empty a b
            }
        "#
    ));

    assert_eq!(actual.out, "false");
}
