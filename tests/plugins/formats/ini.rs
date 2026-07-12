use miette::Diagnostic;
use nu_protocol::test_record;
use nu_test_support::fs::Stub::{FileWithContent, FileWithContentToBeTrimmed};
use nu_test_support::nu_with_plugins;
use nu_test_support::playground::Playground;
use nu_test_support::prelude::*;
use pretty_assertions::assert_eq;

const TEST_CWD: &str = "tests/fixtures/formats";

#[test]
#[deps(NU_PLUGIN_FORMATS)]
fn parses_ini() -> Result {
    test()
        .cwd(TEST_CWD)
        .run("open sample.ini")
        .expect_value_eq(test_record! {
            "SectionOne" => test_record! {
                "key" => "value",
                "integer" => "1234",
                "real" => "3.14",
                "string1" => "Case 1",
                "string2" => "Case 2",
            },
            "SectionTwo" => test_record! {
                "key" => "new value",
                "integer" => "5678",
                "real" => "3.14",
                "string1" => "Case 1",
                "string2" => "Case 2",
                "string3" => "Case 3",
            }
        })
}

#[test]
#[deps(NU_PLUGIN_FORMATS)]
fn parses_utf16_ini() -> Result {
    let code = "
        open ./utf16.ini --raw
        | decode utf-16
        | from ini
        | get '.ShellClassInfo'
        | get IconIndex
    ";
    test().cwd(TEST_CWD).run(code).expect_value_eq("-236")
}

#[test]
#[deps(NU_PLUGIN_FORMATS)]
fn read_ini_with_missing_session() -> Result {
    Playground::setup("from ini with missiong session", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "some_missing.ini",
            "
            min-width=450
            max-width=820
            [normal]
            sound-file=/usr/share/sounds/freedesktop/stereo/dialog-information.oga
            [critical]
            border-color=FAB387ff
            default-timeout=20
            sound-file=/usr/share/sounds/freedesktop/stereo/dialog-warning.oga
            ",
        )]);
        test()
            .cwd(dirs.test())
            .run("open some_missing.ini | get ''.min-width")
            .expect_value_eq("450")
    })
}

#[test]
#[deps(NU_PLUGIN_FORMATS)]
fn read_ini_with_no_escape() -> Result {
    Playground::setup("from ini with no escape", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "windows_path.ini",
            "[start]\nfile=C:\\Windows\\System32\\xcopy.exe\n",
        )]);

        let cwd = dirs.test();

        let err = test()
            .cwd(cwd)
            .run("open windows_path.ini --raw | from ini")
            .expect_shell_error()?;

        let contains_message = err.labels().unwrap().any(|label| {
            label
                .label()
                .is_some_and(|text| text.contains("unknown character in \\xHH form"))
        });

        assert!(contains_message);

        test()
            .cwd(cwd)
            .run("open windows_path.ini --raw | from ini --no-escape | get start.file")
            .expect_value_eq(r"C:\Windows\System32\xcopy.exe")
    })
}

#[test]
#[deps(NU_PLUGIN_FORMATS)]
fn read_ini_with_no_quote() -> Result {
    Playground::setup("from ini with no quote", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent("quoted.ini", "[foo]\nbar='quoted'\n")]);

        let cwd = dirs.test();

        test()
            .cwd(cwd)
            .run("open quoted.ini --raw | from ini | get foo.bar | $in == 'quoted'")
            .expect_value_eq(true)?;

        test()
            .cwd(cwd)
            .run(r#"open quoted.ini --raw | from ini --no-quote | get foo.bar | $in == "'quoted'""#)
            .expect_value_eq(true)
    })
}

#[test]
#[deps(NU_PLUGIN_FORMATS)]
fn read_ini_with_indented_multiline_value() -> Result {
    Playground::setup("from ini with indented multiline", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "multiline.ini",
            "[foo]\nbar=line one\n  line two\n",
        )]);

        let cwd = dirs.test();

        test()
            .cwd(cwd)
            .run("open multiline.ini --raw | from ini | get foo.bar | str contains 'line two'")
            .expect_value_eq(false)?;

        test()
            .cwd(cwd)
            .run("open multiline.ini --raw | from ini -m | get foo.bar | str contains 'line two'")
            .expect_value_eq(true)
    })
}

#[test]
#[deps(NU_PLUGIN_FORMATS)]
fn read_ini_with_preserve_key_leading_whitespace() -> Result {
    Playground::setup("from ini with key whitespace", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "key_whitespace.ini",
            "[foo]\n  key=value\n",
        )]);

        let cwd = dirs.test();

        test()
            .cwd(cwd)
            .run("open key_whitespace.ini --raw | from ini -w | get foo.'  key'")
            .expect_value_eq("value")?;

        test()
            .cwd(cwd)
            .run("open key_whitespace.ini --raw | from ini | get foo.key")
            .expect_value_eq("value")
    })
}
