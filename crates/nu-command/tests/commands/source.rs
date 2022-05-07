use nu_test_support::fs::{AbsolutePath, DisplayPath, Stub::FileWithContent};
use nu_test_support::nu;
use nu_test_support::pipeline;
use nu_test_support::playground::Playground;

#[should_panic]
#[test]
fn sources_also_files_under_custom_lib_dirs_path() {
    Playground::setup("source_test_1", |dirs, nu| {
        let file = AbsolutePath::new(dirs.test().join("config.toml"));
        let library_path = AbsolutePath::new(dirs.test().join("lib"));

        nu.with_config(&file);
        nu.with_files(vec![FileWithContent(
            "config.toml",
            &format!(
                r#"
                lib_dirs = ["{}"]
                skip_welcome_message = true
            "#,
                library_path.display_path()
            ),
        )]);

        nu.within("lib").with_files(vec![FileWithContent(
            "my_library.nu",
            r#"
                source my_library/main.nu
            "#,
        )]);
        nu.within("lib/my_library").with_files(vec![FileWithContent(
            "main.nu",
            r#"
            def hello [] {
                echo "hello nu"
            }
            "#,
        )]);

        let actual = nu!(
            cwd: ".", pipeline(
            r#"
                source my_library.nu ;

                hello
        "#
        ));

        assert_eq!(actual.out, "hello nu");
    })
}

fn try_source_foo_with_double_quotes_in(testdir: &str, playdir: &str) {
    Playground::setup(playdir, |dirs, sandbox| {
        let testdir = String::from(testdir);
        let mut foo_file = testdir.clone();
        foo_file.push_str("/foo.nu");

        sandbox.mkdir(&testdir);
        sandbox.with_files(vec![FileWithContent(&foo_file, "echo foo")]);

        let cmd = String::from("source ") + r#"""# + &foo_file + r#"""#;

        let actual = nu!(cwd: dirs.test(), &cmd);

        assert_eq!(actual.out, "foo");
    });
}

fn try_source_foo_with_single_quotes_in(testdir: &str, playdir: &str) {
    Playground::setup(playdir, |dirs, sandbox| {
        let testdir = String::from(testdir);
        let mut foo_file = testdir.clone();
        foo_file.push_str("/foo.nu");

        sandbox.mkdir(&testdir);
        sandbox.with_files(vec![FileWithContent(&foo_file, "echo foo")]);

        let cmd = String::from("source ") + r#"'"# + &foo_file + r#"'"#;

        let actual = nu!(cwd: dirs.test(), &cmd);

        assert_eq!(actual.out, "foo");
    });
}

fn try_source_foo_without_quotes_in(testdir: &str, playdir: &str) {
    Playground::setup(playdir, |dirs, sandbox| {
        let testdir = String::from(testdir);
        let mut foo_file = testdir.clone();
        foo_file.push_str("/foo.nu");

        sandbox.mkdir(&testdir);
        sandbox.with_files(vec![FileWithContent(&foo_file, "echo foo")]);

        let cmd = String::from("source ") + &foo_file;

        let actual = nu!(cwd: dirs.test(), &cmd);

        assert_eq!(actual.out, "foo");
    });
}

#[test]
fn sources_unicode_file_in_normal_dir() {
    try_source_foo_with_single_quotes_in("foo", "source_test_1");
    try_source_foo_with_double_quotes_in("foo", "source_test_2");
    try_source_foo_without_quotes_in("foo", "source_test_3");
}

#[test]
fn sources_unicode_file_in_unicode_dir_without_spaces_1() {
    try_source_foo_with_single_quotes_in("🚒", "source_test_4");
    try_source_foo_with_double_quotes_in("🚒", "source_test_5");
    try_source_foo_without_quotes_in("🚒", "source_test_6");
}

#[cfg(not(windows))] // ':' is not allowed in Windows paths
#[test]
fn sources_unicode_file_in_unicode_dir_without_spaces_2() {
    try_source_foo_with_single_quotes_in(":fire_engine:", "source_test_7");
    try_source_foo_with_double_quotes_in(":fire_engine:", "source_test_8");
    try_source_foo_without_quotes_in(":fire_engine:", "source_test_9");
}

#[test]
fn sources_unicode_file_in_unicode_dir_with_spaces_1() {
    // this one fails
    try_source_foo_with_single_quotes_in("e-$ èрт🚒♞中片-j", "source_test_8");
    // this one passes
    try_source_foo_with_double_quotes_in("e-$ èрт🚒♞中片-j", "source_test_9");
}

#[cfg(not(windows))] // ':' is not allowed in Windows paths
#[test]
fn sources_unicode_file_in_unicode_dir_with_spaces_2() {
    try_source_foo_with_single_quotes_in("e-$ èрт:fire_engine:♞中片-j", "source_test_10");
    try_source_foo_with_double_quotes_in("e-$ èрт:fire_engine:♞中片-j", "source_test_11");
}

#[ignore]
#[test]
fn sources_unicode_file_in_non_utf8_dir() {
    // How do I create non-UTF-8 path???
}
