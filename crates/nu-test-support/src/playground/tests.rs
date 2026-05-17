use crate::playground::Playground;

#[test]
fn current_working_directory_in_sandbox_directory_created() {
    Playground::setup("topic", |dirs, nu| {
        let original_cwd = dirs.test();
        nu.within("some_directory_within");

        assert_eq!(nu.cwd(), original_cwd.join("some_directory_within"));
    })
}

#[test]
fn current_working_directory_back_to_root_from_anywhere() {
    Playground::setup("topic", |dirs, nu| {
        let original_cwd = dirs.test();

        nu.within("some_directory_within");
        nu.back_to_playground();

        assert_eq!(nu.cwd(), original_cwd);
    })
}
