use nu_test_support::{nu, pipeline};

#[test]
fn fold_table_column() {
    // use nu_test_support::playground::Playground;
    // Playground::setup("str_test_1", |dirs, sandbox| {
    //     sandbox.with_files(vec![FileWithContent(
    //         "sample.toml",
    //         r#"
    //                 [dependency]
    //                 name = "nu "
    //             "#,
    //     )]);
    //
    //     let actual = nu!(
    //         cwd: dirs.test(),
    //         "open sample.toml | str trim dependency.name | get dependency.name | echo $it"
    //     );
    //
    //     assert_eq!(actual.out, "nu");
    // })

    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        echo "[{month:2,total:30}, {month:3,total:10}, {month:4,total:3}, {month:5,total:60}]"
        | from json
        | get total
        | fold 20 { = $it + $( math eval `{{$acc}}^1.05` )}
        | str from -d 1
        "#
        )
    );

    assert_eq!(actual.out, "180.6");

    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        echo "[{month:2,total:30}, {month:3,total:10}, {month:4,total:3}, {month:5,total:60}]"
        | from json
        | fold 20 { = $it.total + $( math eval `{{$acc}}^1.05` )}
        | str from -d 1
        "#
        )
    );

    assert_eq!(actual.out, "180.6");
}

#[test]
fn error_fold_type_mismatch() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        echo 1 2 'whoops' | fold 0 { = $acc + $it }
        "#
        )
    );

    assert!(actual.err.contains("Coercion"));
}
