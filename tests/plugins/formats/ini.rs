use nu_test_support::fs::Stub::{FileWithContent, FileWithContentToBeTrimmed};
use nu_test_support::nu_with_plugins;
use nu_test_support::playground::Playground;
use pretty_assertions::assert_eq;

const TEST_CWD: &str = "tests/fixtures/formats";

#[test]
fn parses_ini() {
    let actual = nu_with_plugins!(
        cwd: TEST_CWD,
        plugin: ("nu_plugin_formats"),
        "open sample.ini | to nuon"
    );

    assert_eq!(
        actual.out,
        r#"{SectionOne: {key: value, integer: "1234", real: "3.14", "string1": "Case 1", "string2": "Case 2"}, SectionTwo: {key: "new value", integer: "5678", real: "3.14", "string1": "Case 1", "string2": "Case 2", "string3": "Case 3"}}"#
    )
}

#[test]
fn parses_utf16_ini() {
    let actual = nu_with_plugins!(
        cwd: TEST_CWD,
        plugin: ("nu_plugin_formats"),
        "open ./utf16.ini --raw | decode utf-16 | from ini | get '.ShellClassInfo' | get IconIndex"
    );

    assert_eq!(actual.out, "-236")
}

#[test]
fn read_ini_with_missing_session() {
    Playground::setup("from ini with missiong session", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "some_missing.ini",
            r#"
            min-width=450
            max-width=820
            [normal]
            sound-file=/usr/share/sounds/freedesktop/stereo/dialog-information.oga
            [critical]
            border-color=FAB387ff
            default-timeout=20
            sound-file=/usr/share/sounds/freedesktop/stereo/dialog-warning.oga
            "#,
        )]);

        let cwd = dirs.test();
        let actual = nu_with_plugins!(
            cwd: cwd,
            plugin: ("nu_plugin_formats"),
            r#"open some_missing.ini | get "".min-width "#
        );

        assert_eq!(actual.out, "450");
    })
}

#[test]
fn read_ini_with_no_escape() {
    Playground::setup("from ini with no escape", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "windows_path.ini",
            "[start]\nfile=C:\\Windows\\System32\\xcopy.exe\n",
        )]);

        let cwd = dirs.test();
        let default = nu_with_plugins!(
            cwd: cwd,
            plugin: ("nu_plugin_formats"),
            "open windows_path.ini --raw | from ini"
        );
        assert!(default.err.contains("unknown character in \\xHH form"));

        let actual = nu_with_plugins!(
            cwd: cwd,
            plugin: ("nu_plugin_formats"),
            "open windows_path.ini --raw | from ini --no-escape | get start.file"
        );

        assert_eq!(actual.out, r"C:\Windows\System32\xcopy.exe");
    })
}

#[test]
fn read_ini_with_no_quote() {
    Playground::setup("from ini with no quote", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent("quoted.ini", "[foo]\nbar='quoted'\n")]);

        let cwd = dirs.test();
        let default = nu_with_plugins!(
            cwd: cwd,
            plugin: ("nu_plugin_formats"),
            "open quoted.ini --raw | from ini | get foo.bar | $in == 'quoted'"
        );
        assert_eq!(default.out, "true");

        let no_quote = nu_with_plugins!(
            cwd: cwd,
            plugin: ("nu_plugin_formats"),
            "open quoted.ini --raw | from ini --no-quote | get foo.bar | $in == \"'quoted'\""
        );
        assert_eq!(no_quote.out, "true");
    })
}

#[test]
fn read_ini_with_indented_multiline_value() {
    Playground::setup("from ini with indented multiline", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "multiline.ini",
            "[foo]\nbar=line one\n  line two\n",
        )]);

        let cwd = dirs.test();
        let default = nu_with_plugins!(
            cwd: cwd,
            plugin: ("nu_plugin_formats"),
            "open multiline.ini --raw | from ini | get foo.bar | str contains 'line two'"
        );
        assert_ne!(default.out, "true");

        let multiline = nu_with_plugins!(
            cwd: cwd,
            plugin: ("nu_plugin_formats"),
            "open multiline.ini --raw | from ini -m | get foo.bar | str contains 'line two'"
        );
        assert_eq!(multiline.out, "true");
    })
}

#[test]
fn read_ini_with_preserve_key_leading_whitespace() {
    Playground::setup("from ini with key whitespace", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "key_whitespace.ini",
            "[foo]\n  key=value\n",
        )]);

        let cwd = dirs.test();
        let default = nu_with_plugins!(
            cwd: cwd,
            plugin: ("nu_plugin_formats"),
            "open key_whitespace.ini --raw | from ini | get foo.key"
        );
        assert_eq!(default.out, "value");

        let keep = nu_with_plugins!(
            cwd: cwd,
            plugin: ("nu_plugin_formats"),
            "open key_whitespace.ini --raw | from ini -w | get foo.'  key'"
        );
        assert_eq!(keep.out, "value");
    })
}
