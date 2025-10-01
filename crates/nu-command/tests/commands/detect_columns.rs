use nu_test_support::{nu, pipeline, playground::Playground};

#[test]
fn detect_columns_with_legacy() {
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
fn detect_columns_with_legacy_and_flag_c() {
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

#[test]
fn detect_columns_with_flag_c() {
    let body = "$\"
total 284K(char nl)
drwxr-xr-x  2 root root 4.0K Mar 20 08:28 =(char nl)
drwxr-xr-x  4 root root 4.0K Mar 20 08:18 ~(char nl)
-rw-r--r--  1 root root 3.0K Mar 20 07:23 ~asdf(char nl)\"";
    let expected = "[
['column0', 'column1', 'column2', 'column3', 'column4', 'column5', 'column7', 'column8'];
['drwxr-xr-x', '2', 'root', 'root', '4.0K', 'Mar 20', '08:28', '='],
['drwxr-xr-x', '4', 'root', 'root', '4.0K', 'Mar 20', '08:18', '~'],
['-rw-r--r--',  '1', 'root', 'root', '3.0K', 'Mar 20', '07:23', '~asdf']
]";
    let range = "5..6";
    let cmd = format!(
        "({} | detect columns -c {} -s 1 --no-headers) == {}",
        pipeline(body),
        range,
        pipeline(expected),
    );
    println!("debug cmd: {cmd}");
    Playground::setup("detect_columns_test_1", |dirs, _| {
        let out = nu!(
            cwd: dirs.test(),
            cmd,
        );
        println!("{}", out.out);
        assert_eq!(out.out, "true");
    })
}

#[test]
fn detect_columns_may_fail() {
    let out =
        nu!(r#""meooooow cat\nkitty kitty woof" | try { detect columns } catch { "failed" }"#);
    assert_eq!(out.out, "failed");
}
