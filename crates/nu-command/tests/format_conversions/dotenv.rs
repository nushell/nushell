use nu_test_support::{fs::Stub::FileWithContent, playground::Playground, prelude::*};
use std::fs;

#[test]
fn record_round_trips() -> Result {
    let record = test_record! {
        "PLAIN" => "/usr/bin:/bin",
        "EMPTY" => "",
        "BLANKS" => "  two words  ",
        "COMMENT" => "a #b",
        "VAR" => "${HOME}/x",
        "QUOTES" => r#"it's "quoted" ${HOME} \ $5"#,
        "BARE" => "it's $HOME and $5 and $(whoami)",
        "SINGLE" => "$HOME",
        "LINES" => "one\ntwo\r\nthree\tfour",
        "UNICODE" => "Grüße 日本",
        "dotted.name-1" => "x",
    };

    test()
        .run_with_data("$in | to dotenv | from dotenv", record.clone())
        .expect_value_eq(record)
}

#[test]
fn values_are_written_as_externals_see_them() -> Result {
    // A `;` isn't plain, so Windows' joined PATH gets quotes.
    let path = if cfg!(windows) { "'/a;/b'" } else { "/a:/b" };

    test()
        .run("{PORT: 8080, DEBUG: true, PATH: [/a /b]} | to dotenv")
        .expect_value_eq(format!("PORT=8080\nDEBUG=true\nPATH={path}\n"))
}

#[test]
fn values_without_text_are_errors() -> Result {
    let err = test().run("{A: {b: 1}} | to dotenv").expect_shell_error()?;
    assert_contains("Can't write `A` as .env text", err.to_string());

    let err = test().run("{'1A': x} | to dotenv").expect_shell_error()?;
    assert_contains("Can't write `1A` as .env text", err.to_string());
    Ok(())
}

#[test]
fn open_reads_dotenv_file_names() -> Result {
    Playground::setup("dotenv_open", |dirs, sandbox| {
        sandbox.with_files(&[
            FileWithContent(".env", "A=1\n"),
            FileWithContent(".env.local", "A=2\n"),
            FileWithContent("prod.env", "A=3\n"),
            FileWithContent(".env.json", r#"{"A": 4}"#),
        ]);

        test()
            .cwd(dirs.test())
            .run("[.env .env.local prod.env .env.json] | each { open $in | get A }")
            .expect_value_eq(test_value!(["1", "2", "3", 4]))?;
        test()
            .cwd(dirs.test())
            .run("open --raw .env")
            .expect_value_eq("A=1\n")
    })
}

#[test]
fn save_writes_dotenv_file_names() -> Result {
    Playground::setup("dotenv_save", |dirs, sandbox| {
        sandbox.with_files(&[]);

        let () = test()
            .cwd(dirs.test())
            .run("[.env .env.local prod.env] | each {|file| {A: 'x y'} | save $file } | ignore")?;
        for file in [".env", ".env.local", "prod.env"] {
            assert_eq!(fs::read_to_string(dirs.test().join(file))?, "A='x y'\n");
        }

        // A real extension keeps its own format.
        let () = test().cwd(dirs.test()).run("{A: 1} | save .env.json")?;
        assert_contains(
            r#""A": 1"#,
            fs::read_to_string(dirs.test().join(".env.json"))?,
        );
        Ok(())
    })
}

#[test]
fn open_and_load_env_leave_path_a_list() -> Result {
    Playground::setup("dotenv_load_env", |dirs, sandbox| {
        let separator = if cfg!(windows) { ";" } else { ":" };
        let text = format!("PATH=/a{separator}/b\nNAME='my app'\n");
        sandbox.with_files(&[FileWithContent(".env", &text)]);

        test()
            .cwd(dirs.test())
            .run("open .env | load-env; [$env.PATH $env.NAME]")
            .expect_value_eq(test_value!([["/a", "/b"], "my app"]))
    })
}

#[test]
fn null_values_are_written_empty() -> Result {
    test()
        .run("{A: null, B: 1} | to dotenv")
        .expect_value_eq("A=\nB=1\n")
}

#[test]
fn keys_ignore_case_and_keep_their_first_spelling() -> Result {
    test()
        .run(r#""A=1\na=2\nB=${A}" | from dotenv | to nuon"#)
        .expect_value_eq(r#"{A: "2", B: "2"}"#)
}

#[test]
fn save_writes_other_values_to_dotenv_file_names_as_before() -> Result {
    Playground::setup("dotenv_save_list", |dirs, sandbox| {
        sandbox.with_files(&[]);

        let () = test().cwd(dirs.test()).run("[a b] | save .env")?;
        assert_eq!(fs::read_to_string(dirs.test().join(".env"))?, "a\nb\n");
        Ok(())
    })
}

#[test]
fn save_and_open_agree_on_extension_case() -> Result {
    Playground::setup("dotenv_save_case", |dirs, sandbox| {
        sandbox.with_files(&[]);

        test()
            .cwd(dirs.test())
            .run("{A: 1} | save .env.JSON; open .env.JSON | get A")
            .expect_value_eq(1)
    })
}

#[test]
fn open_reads_a_dotenv_file_name_that_isnt_text_as_raw_data() -> Result {
    Playground::setup("dotenv_open_binary", |dirs, sandbox| {
        sandbox.with_files(&[]);
        fs::write(dirs.test().join(".env.gpg"), [0xff, 0xfe, 0x00])?;

        test()
            .cwd(dirs.test())
            .run("open .env.gpg | into binary")
            .expect_value_eq(Value::test_binary([0xff, 0xfe, 0x00]))
    })
}

#[test]
fn a_shell_script_gets_a_hint() -> Result {
    test()
        .run(
            r#"try { "source venv/bin/activate" | from dotenv } catch {|err| $err.details.help }"#,
        )
        .expect_value_eq(
            "a .env file holds `KEY=value` lines; a shell script, such as an autoenv `.env` file, isn't one",
        )
}

#[test]
fn bare_names_expand_like_braced_ones() -> Result {
    test()
        .run(r#"$env.HOME_DIR = "/home/me"; "export PATH=$HOME_DIR/bin:${HOME_DIR}/sbin" | from dotenv | get PATH"#)
        .expect_value_eq("/home/me/bin:/home/me/sbin")
}
