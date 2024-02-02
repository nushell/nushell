use nu_test_support::nu;
use nu_test_support::playground::Playground;
#[test]
fn test_uname_all() {
    Playground::setup("uname_test_1", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(),
            "uname"
        );
        assert!(actual.status.success())
    })
}
