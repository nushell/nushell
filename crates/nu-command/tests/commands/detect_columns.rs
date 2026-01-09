use nu_test_support::{nu, playground::Playground};

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
                format!(
                    "({} | detect columns) == {}",
                    case.0,
                    case.1
                )
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
                format!(
                    "({} | detect columns --combine-columns {}) == {}",
                    case.0,
                    case.2,
                    case.1,
                )
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
    let body = r#""total 284K
drwxr-xr-x  2 root root 4.0K Mar 20 08:28 =
drwxr-xr-x  4 root root 4.0K Mar 20 08:18 ~
-rw-r--r--  1 root root 3.0K Mar 20 07:23 ~asdf
""#;

    let expected = r#"[
    ['column0', 'column1', 'column2', 'column3', 'column4', 'column5', 'column7', 'column8'];
    ['drwxr-xr-x', '2', 'root', 'root', '4.0K', 'Mar 20', '08:28', '='],
    ['drwxr-xr-x', '4', 'root', 'root', '4.0K', 'Mar 20', '08:18', '~'],
    ['-rw-r--r--',  '1', 'root', 'root', '3.0K', 'Mar 20', '07:23', '~asdf']
]"#;

    let range = "5..6";
    let cmd = format!("({body} | detect columns -c {range} -s 1 --no-headers) == {expected}",);
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
    // Test case where column detection produces duplicate column names.
    // With our updated implementation, when detection fails due to mismatched
    // columns, data goes to "data" column instead of throwing an error.
    // But duplicate column headers still cause an error.
    let out = nu!(r#""cat cat\nkitty woof" | try { detect columns } catch { "failed" }"#);
    assert_eq!(out.out, "failed");
}

#[test]
fn detect_columns_preserves_original_content_on_mismatch() {
    // Test with iptab-like output containing box drawing characters.
    // When column detection fails (headers don't match data rows),
    // all rows should be output in a consistent "data" column,
    // preserving the original content including box characters.
    let iptab_sample = r#""+----------------------------------------------+
| addrs   bits   pref   class  mask            |
+----------------------------------------------+
|     1      0    /32          255.255.255.255 |
|     2      1    /31          255.255.255.254 |
+----------------------------------------------+""#;

    // All rows should be in the "data" column when detection fails (6 lines total)
    let out = nu!(format!(
        r#"{} | detect columns | get data | length"#,
        iptab_sample
    ));
    assert_eq!(out.out, "6", "All rows should be in the data column");

    // The "data" column should contain the full original text, including 'addrs'
    let out = nu!(format!(
        r#"{} | detect columns | get data | any {{|l| $l | str contains "addrs"}}"#,
        iptab_sample
    ));
    assert_eq!(
        out.out, "true",
        "Line data should preserve original content including 'addrs'"
    );

    // Verify the box-only lines still have box characters (+ and -)
    let out2 = nu!(format!(
        r#"{} | detect columns | get data | any {{|l| ($l | str contains "+") and ($l | str contains "-")}}"#,
        iptab_sample
    ));
    assert_eq!(
        out2.out, "true",
        "Box-only lines should preserve + and - characters"
    );

    // Verify data lines preserve the pipe | characters
    let out3 = nu!(format!(
        r#"{} | detect columns | get data | where {{|l| $l | str contains "addrs"}} | first | str contains "|""#,
        iptab_sample
    ));
    assert_eq!(
        out3.out, "true",
        "Data lines should preserve pipe | characters"
    );
}

#[test]
fn detect_columns_ignore_box_chars_flag() {
    // When --ignore-box-chars is used, lines consisting entirely of box drawing
    // characters should be ignored

    // Simple test: header line is good, separator line is ignored
    let simple_sample = r#""col1 col2 col3
----+----+----
val1 val2 val3""#;

    // Without flag: separator line causes column mismatch, so all rows go to "data"
    // (including the first line which is used as header attempt)
    let out = nu!(format!(
        r#"{} | detect columns | get data? | length"#,
        simple_sample
    ));
    // Header is "col1 col2 col3" (3 cols), separator is "----+----+----" (1 col),
    // Since first data row (separator) doesn't match header, all 3 rows go to "data"
    assert_eq!(
        out.out, "3",
        "Without --ignore-box-chars, all rows go to data column when detection fails"
    );

    // With --ignore-box-chars flag: the separator line is ignored
    let out2 = nu!(format!(
        r#"{} | detect columns --ignore-box-chars | get col1 | first"#,
        simple_sample
    ));
    assert_eq!(
        out2.out, "val1",
        "With --ignore-box-chars, separator is ignored and columns work"
    );
}
