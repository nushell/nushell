use nu_test_support::playground::Playground;
use nu_test_support::nu;

#[test]
fn first_and_false() {
    Playground::setup("create_test_1", |dirs, _sandbox| {
        let actual = nu!(
            cwd: dirs.test(),
            "false && touch i_will_not_be_created_amd.txt && echo done"
        );

        let path = dirs.test().join("i_will_not_be_created_and.txt");
        assert!(!path.exists());
        assert_eq!(actual.out, "false");
    })
}

#[test]
fn first_and_true() {
    Playground::setup("create_test_2", |dirs, _sandbox| {
        let actual = nu!(
            cwd: dirs.test(),
            "true && touch i_will_be_created_and.txt && echo done"
        );

        let path = dirs.test().join("i_will_be_created_and.txt");
        assert!(path.exists());
        assert_eq!(actual.out, "truedone");
    })
}

