use nu_test_support::fs::Stub::FileWithContent;
use nu_test_support::playground::Playground;
use nu_test_support::nu;

fn try_source_foo_with_quotes_in(testdir: &str) {
    Playground::setup("source_test_1", |dirs, sandbox| {
        let testdir = String::from(testdir);
        let mut foo_file = testdir.clone();
        foo_file.push_str("/🚒.nu");

        sandbox.mkdir(&testdir);
        sandbox.with_files(vec![
            FileWithContent(
                &foo_file,
                "echo foo",
            )
        ]);

        let cmd = String::from("source ") + r#"""# + &foo_file + r#"""#;

        let actual = nu!(cwd: dirs.test(), &cmd);

        assert_eq!(actual.out, "foo");
    });
}

fn try_source_foo_without_quotes_in(testdir: &str) {
    Playground::setup("source_test_1", |dirs, sandbox| {
        let testdir = String::from(testdir);
        let mut foo_file = testdir.clone();
        foo_file.push_str("/🚒.nu");

        sandbox.mkdir(&testdir);
        sandbox.with_files(vec![
            FileWithContent(
                &foo_file,
                "echo foo",
            )
        ]);

        let cmd = String::from("source ") + &foo_file;

        let actual = nu!(cwd: dirs.test(), &cmd);

        assert_eq!(actual.out, "foo");
    });
}

#[test]
fn sources_unicode_file_in_normal_dir() {
    try_source_foo_with_quotes_in("foo");
    try_source_foo_without_quotes_in("foo");
}

#[test]
fn sources_unicode_file_in_unicode_dir_without_spaces_1() {
    try_source_foo_with_quotes_in("🚒");
    try_source_foo_without_quotes_in("🚒");
}

#[test]
fn sources_unicode_file_in_unicode_dir_without_spaces_2() {
    try_source_foo_with_quotes_in(":fire_engine:");
    try_source_foo_without_quotes_in(":fire_engine:");
}

#[test]
fn sources_unicode_file_in_unicode_dir_with_spaces_1() {
    try_source_foo_with_quotes_in("e-$ èрт🚒♞中片-j");
}

#[test]
fn sources_unicode_file_in_unicode_dir_with_spaces_2() {
    try_source_foo_with_quotes_in("e-$ èрт:fire_engine:♞中片-j");
}

#[ignore]
#[test]
fn sources_unicode_file_in_non_utf8_dir() {
    // How do I create non-UTF-8 path???
}
