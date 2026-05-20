use nu_test_support::prelude::*;

#[test]
fn reports_emptiness() -> Result {
    let code = "
        [[] '' {} null]
        | all {|| is-empty }
    ";

    test().run(code).expect_value_eq(true)
}

#[test]
fn reports_nonemptiness() -> Result {
    let code = "
        [[1] ' ' {a:1} 0]
        | any {|| is-empty }
    ";

    test().run(code).expect_value_eq(false)
}

#[test]
fn reports_emptiness_by_columns() -> Result {
    let code = "
        [{a:1 b:null c:null} {a:2 b:null c:null}]
        | any {|| is-empty b c }
    ";

    test().run(code).expect_value_eq(true)
}

#[test]
fn reports_nonemptiness_by_columns() -> Result {
    let code = "
        [{a:1 b:null c:3} {a:null b:5 c:2}]
        | any {|| is-empty a b }
    ";

    test().run(code).expect_value_eq(false)
}
