use nu_test_support::{nu, playground::Playground};

#[test]
fn detect_columns() {
    let cases = [(
        "$\"c1 c2 c3 c4 c5(char nl)a b c d e\"",
        "[[c1,c2,c3,c4,c5]; [a,b,c,d,e]]",
    )];

    Playground::setup("detect_columns_test_1", |dirs, _| {
        for case in cases.into_iter() {
            let out = nu!(
                cwd: dirs.test(),
                "({} | detect columns) == {}",
                case.0,
                case.1
            );

            assert_eq!(
                out.out, "true",
                "({} | detect columns) == {}",
                case.0, case.1
            );
        }
    });
}

#[test]
fn detect_columns_with_flag_c() {
    let cases = [
        (
            "$\"c1 c2 c3 c4 c5(char nl)a b c d e\"",
            "[[c1,c3,c4,c5]; ['a b',c,d,e]]",
            "0..1",
        ),
        (
            "$\"c1 c2 c3 c4 c5(char nl)a b c d e\"",
            "[[c1,c2,c3,c4]; [a,b,c,'d e']]",
            "(-2)..(-1)",
        ),
        (
            "$\"c1 c2 c3 c4 c5(char nl)a b c d e\"",
            "[[c1,c2,c3]; [a,b,'c d e']]",
            "2..",
        ),
    ];

    Playground::setup("detect_columns_test_1", |dirs, _| {
        for case in cases.into_iter() {
            let out = nu!(
                cwd: dirs.test(),
                "({} | detect columns --combine-columns {}) == {}",
                case.0,
                case.2,
                case.1,
            );

            assert_eq!(
                out.out, "true",
                "({} | detect columns --combine-columns {}) == {}",
                case.0, case.2, case.1
            );
        }
    });
}
