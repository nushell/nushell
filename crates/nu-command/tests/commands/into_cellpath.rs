use nu_test_support::{nu, pipeline};

#[test]
fn into_pathcell_string() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        'nu.is.awesome' | into cellpath
        "#
    ));
    dbg!(&actual.out);

    assert!(actual.out.contains("nu.is.awesome"));
}
