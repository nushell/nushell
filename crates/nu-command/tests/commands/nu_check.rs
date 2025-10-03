use nu_test_support::fs::Stub::FileWithContentToBeTrimmed;
use nu_test_support::nu;
use nu_test_support::playground::Playground;

#[test]
fn parse_script_success() {
    Playground::setup("nu_check_test_1", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "script.nu",
            r#"
                greet "world"

                def greet [name] {
                  echo "hello" $name
                }
            "#,
        )]);

        let actual = nu!(cwd: dirs.test(), "
            nu-check script.nu
        ");

        assert!(actual.err.is_empty());
    })
}

#[test]
fn parse_script_with_wrong_type() {
    Playground::setup("nu_check_test_2", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "script.nu",
            r#"
                greet "world"

                def greet [name] {
                  echo "hello" $name
                }
            "#,
        )]);

        let actual = nu!(cwd: dirs.test(), "
            nu-check --debug --as-module script.nu
        ");

        assert!(actual.err.contains("Failed to parse content"));
    })
}
#[test]
fn parse_script_failure() {
    Playground::setup("nu_check_test_3", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "script.nu",
            r#"
                greet "world"

                def greet [name {
                  echo "hello" $name
                }
            "#,
        )]);

        let actual = nu!(cwd: dirs.test(), "
            nu-check --debug script.nu
        ");

        assert!(actual.err.contains("Unexpected end of code"));
    })
}

#[test]
fn parse_module_success() {
    Playground::setup("nu_check_test_4", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "foo.nu",
            r#"
                # foo.nu

                export def hello [name: string] {
                    $"hello ($name)!"
                }

                export def hi [where: string] {
                    $"hi ($where)!"
                }
            "#,
        )]);

        let actual = nu!(cwd: dirs.test(), "
            nu-check --as-module foo.nu
        ");

        assert!(actual.err.is_empty());
    })
}

#[test]
fn parse_module_with_wrong_type() {
    Playground::setup("nu_check_test_5", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "foo.nu",
            r#"
                # foo.nu

                export def hello [name: string {
                    $"hello ($name)!"
                }

                export def hi [where: string] {
                    $"hi ($where)!"
                }
            "#,
        )]);

        let actual = nu!(cwd: dirs.test(), "
            nu-check --debug foo.nu
        ");

        assert!(actual.err.contains("Failed to parse content"));
    })
}
#[test]
fn parse_module_failure() {
    Playground::setup("nu_check_test_6", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "foo.nu",
            r#"
                # foo.nu

                export def hello [name: string {
                    $"hello ($name)!"
                }

                export def hi [where: string] {
                    $"hi ($where)!"
                }
            "#,
        )]);

        let actual = nu!(cwd: dirs.test(), "
            nu-check --debug --as-module foo.nu
        ");

        assert!(actual.err.contains("Unexpected end of code"));
    })
}

#[test]
fn file_not_exist() {
    Playground::setup("nu_check_test_7", |dirs, _sandbox| {
        let actual = nu!(cwd: dirs.test(), "
            nu-check --as-module foo.nu
        ");

        assert!(actual.err.contains("nu::shell::io::file_not_found"));
    })
}

#[test]
fn parse_module_success_2() {
    Playground::setup("nu_check_test_10", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "foo.nu",
            r#"
                # foo.nu

                export-env { $env.MYNAME = "Arthur, King of the Britons" }
            "#,
        )]);

        let actual = nu!(cwd: dirs.test(), "
            nu-check --as-module foo.nu
        ");

        assert!(actual.err.is_empty());
    })
}

#[test]
fn parse_script_success_with_raw_stream() {
    Playground::setup("nu_check_test_11", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "script.nu",
            r#"
                greet "world"

                def greet [name] {
                  echo "hello" $name
                }
            "#,
        )]);

        let actual = nu!(cwd: dirs.test(), "
            open script.nu | nu-check
        ");

        assert!(actual.err.is_empty());
    })
}

#[test]
fn parse_module_success_with_raw_stream() {
    Playground::setup("nu_check_test_12", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "foo.nu",
            r#"
                # foo.nu

                export def hello [name: string] {
                    $"hello ($name)!"
                }

                export def hi [where: string] {
                    $"hi ($where)!"
                }
            "#,
        )]);

        let actual = nu!(cwd: dirs.test(), "
            open foo.nu | nu-check --as-module
        ");

        assert!(actual.err.is_empty());
    })
}

#[test]
fn parse_string_as_script_success() {
    Playground::setup("nu_check_test_13", |dirs, _sandbox| {
        let actual = nu!(cwd: dirs.test(), r#"
            echo $'two(char nl)lines' | nu-check
        "#);

        assert!(actual.err.is_empty());
    })
}

#[test]
fn parse_string_as_script() {
    Playground::setup("nu_check_test_14", |dirs, _sandbox| {
        let actual = nu!(cwd: dirs.test(), r#"
            echo $'two(char nl)lines' | nu-check --debug --as-module
        "#);

        println!("the output is {}", actual.err);
        assert!(actual.err.contains("Failed to parse content"));
    })
}

#[test]
fn parse_module_success_with_internal_stream() {
    Playground::setup("nu_check_test_15", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "foo.nu",
            r#"
                # foo.nu

                export def hello [name: string] {
                    $"hello ($name)!"
                }

                export def hi [where: string] {
                    $"hi ($where)!"
                }
            "#,
        )]);

        let actual = nu!(cwd: dirs.test(), "
            open foo.nu | lines | nu-check --as-module
        ");

        assert!(actual.err.is_empty());
    })
}

#[test]
fn parse_script_success_with_complex_internal_stream() {
    Playground::setup("nu_check_test_16", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "grep.nu",
            r#"
                #grep for nu
                def grep-nu [
                  search   #search term
                  entrada?  #file or pipe
                  #
                  #Examples
                  #grep-nu search file.txt
                  #ls **/* | some_filter | grep-nu search
                  #open file.txt | grep-nu search
                ] {
                  if ($entrada | is-empty) {
                    if ($in | column? name) {
                      grep -ihHn $search ($in | get name)
                    } else {
                      ($in | into string) | grep -ihHn $search
                    }
                  } else {
                      grep -ihHn $search $entrada
                  }
                  | lines
                  | parse "{file}:{line}:{match}"
                  | str trim
                  | update match {|f|
                      $f.match
                      | nu-highlight
                    }
                  | rename "source file" "line number"
                }

            "#,
        )]);

        let actual = nu!(cwd: dirs.test(), "
            open grep.nu | lines | nu-check
        ");

        assert!(actual.err.is_empty());
    })
}

#[test]
fn parse_script_failure_with_complex_internal_stream() {
    Playground::setup("nu_check_test_17", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "grep.nu",
            r#"
                #grep for nu
                def grep-nu [
                  search   #search term
                  entrada?  #file or pipe
                  #
                  #Examples
                  #grep-nu search file.txt
                  #ls **/* | some_filter | grep-nu search
                  #open file.txt | grep-nu search
                ]
                  if ($entrada | is-empty) {
                    if ($in | column? name) {
                      grep -ihHn $search ($in | get name)
                    } else {
                      ($in | into string) | grep -ihHn $search
                    }
                  } else {
                      grep -ihHn $search $entrada
                  }
                  | lines
                  | parse "{file}:{line}:{match}"
                  | str trim
                  | update match {|f|
                      $f.match
                      | nu-highlight
                    }
                  | rename "source file" "line number"
                }

            "#,
        )]);

        let actual = nu!(cwd: dirs.test(), "
            open grep.nu | lines | nu-check
        ");

        assert_eq!(actual.out, "false".to_string());
    })
}

#[test]
fn parse_script_success_with_complex_external_stream() {
    Playground::setup("nu_check_test_18", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "grep.nu",
            r#"
                #grep for nu
                def grep-nu [
                  search   #search term
                  entrada?  #file or pipe
                  #
                  #Examples
                  #grep-nu search file.txt
                  #ls **/* | some_filter | grep-nu search
                  #open file.txt | grep-nu search
                ] {
                  if ($entrada | is-empty) {
                    if ($in | column? name) {
                      grep -ihHn $search ($in | get name)
                    } else {
                      ($in | into string) | grep -ihHn $search
                    }
                  } else {
                      grep -ihHn $search $entrada
                  }
                  | lines
                  | parse "{file}:{line}:{match}"
                  | str trim
                  | update match {|f|
                      $f.match
                      | nu-highlight
                    }
                  | rename "source file" "line number"
                }

            "#,
        )]);

        let actual = nu!(cwd: dirs.test(), "
            open grep.nu | nu-check
        ");

        assert!(actual.err.is_empty());
    })
}

#[test]
fn parse_module_success_with_complex_external_stream() {
    Playground::setup("nu_check_test_19", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "grep.nu",
            r#"
                #grep for nu
                def grep-nu [
                  search   #search term
                  entrada?  #file or pipe
                  #
                  #Examples
                  #grep-nu search file.txt
                  #ls **/* | some_filter | grep-nu search
                  #open file.txt | grep-nu search
                ] {
                  if ($entrada | is-empty) {
                    if ($in | column? name) {
                      grep -ihHn $search ($in | get name)
                    } else {
                      ($in | into string) | grep -ihHn $search
                    }
                  } else {
                      grep -ihHn $search $entrada
                  }
                  | lines
                  | parse "{file}:{line}:{match}"
                  | str trim
                  | update match {|f|
                      $f.match
                      | nu-highlight
                    }
                  | rename "source file" "line number"
                }

            "#,
        )]);

        let actual = nu!(cwd: dirs.test(), "
            open grep.nu | nu-check --debug --as-module
        ");

        assert!(actual.err.is_empty());
    })
}

#[test]
fn parse_with_flag_success_for_complex_external_stream() {
    Playground::setup("nu_check_test_20", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "grep.nu",
            r#"
                #grep for nu
                def grep-nu [
                  search   #search term
                  entrada?  #file or pipe
                  #
                  #Examples
                  #grep-nu search file.txt
                  #ls **/* | some_filter | grep-nu search
                  #open file.txt | grep-nu search
                ] {
                  if ($entrada | is-empty) {
                    if ($in | column? name) {
                      grep -ihHn $search ($in | get name)
                    } else {
                      ($in | into string) | grep -ihHn $search
                    }
                  } else {
                      grep -ihHn $search $entrada
                  }
                  | lines
                  | parse "{file}:{line}:{match}"
                  | str trim
                  | update match {|f|
                      $f.match
                      | nu-highlight
                    }
                  | rename "source file" "line number"
                }

            "#,
        )]);

        let actual = nu!(cwd: dirs.test(), "
            open grep.nu | nu-check --debug
        ");

        assert!(actual.err.is_empty());
    })
}

#[test]
fn parse_with_flag_failure_for_complex_external_stream() {
    Playground::setup("nu_check_test_21", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "grep.nu",
            r#"
                #grep for nu
                def grep-nu
                  search   #search term
                  entrada?  #file or pipe
                  #
                  #Examples
                  #grep-nu search file.txt
                  #ls **/* | some_filter | grep-nu search
                  #open file.txt | grep-nu search
                ] {
                  if ($entrada | is-empty) {
                    if ($in | column? name) {
                      grep -ihHn $search ($in | get name)
                    } else {
                      ($in | into string) | grep -ihHn $search
                    }
                  } else {
                      grep -ihHn $search $entrada
                  }
                  | lines
                  | parse "{file}:{line}:{match}"
                  | str trim
                  | update match {|f|
                      $f.match
                      | nu-highlight
                    }
                  | rename "source file" "line number"
                }

            "#,
        )]);

        let actual = nu!(cwd: dirs.test(), "
            open grep.nu | nu-check --debug
        ");

        assert!(actual.err.contains("Failed to parse content"));
    })
}

#[test]
fn parse_with_flag_failure_for_complex_list_stream() {
    Playground::setup("nu_check_test_22", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "grep.nu",
            r#"
                #grep for nu
                def grep-nu
                  search   #search term
                  entrada?  #file or pipe
                  #
                  #Examples
                  #grep-nu search file.txt
                  #ls **/* | some_filter | grep-nu search
                  #open file.txt | grep-nu search
                ] {
                  if ($entrada | is-empty) {
                    if ($in | column? name) {
                      grep -ihHn $search ($in | get name)
                    } else {
                      ($in | into string) | grep -ihHn $search
                    }
                  } else {
                      grep -ihHn $search $entrada
                  }
                  | lines
                  | parse "{file}:{line}:{match}"
                  | str trim
                  | update match {|f|
                      $f.match
                      | nu-highlight
                    }
                  | rename "source file" "line number"
                }

            "#,
        )]);

        let actual = nu!(cwd: dirs.test(), "
            open grep.nu | lines | nu-check --debug
        ");

        assert!(actual.err.contains("Failed to parse content"));
    })
}

#[test]
fn parse_script_with_nested_scripts_success() {
    Playground::setup("nu_check_test_24", |dirs, sandbox| {
        sandbox
            .mkdir("lol")
            .with_files(&[FileWithContentToBeTrimmed(
                "lol/lol.nu",
                r#"
                    source-env ../foo.nu
                    use lol_shell.nu
                    overlay use ../lol/lol_shell.nu
                "#,
            )])
            .with_files(&[FileWithContentToBeTrimmed(
                "lol/lol_shell.nu",
                r#"
                    export def ls [] { "lol" }
                "#,
            )])
            .with_files(&[FileWithContentToBeTrimmed(
                "foo.nu",
                r#"
                    $env.FOO = 'foo'
                "#,
            )]);

        let actual = nu!(cwd: dirs.test(), "
            nu-check lol/lol.nu
        ");

        assert_eq!(actual.out, "true");
    })
}

#[test]
fn nu_check_respects_file_pwd() {
    Playground::setup("nu_check_test_25", |dirs, sandbox| {
        sandbox
            .mkdir("lol")
            .with_files(&[FileWithContentToBeTrimmed(
                "lol/lol.nu",
                r#"
                    $env.RETURN = (nu-check ../foo.nu)
                "#,
            )])
            .with_files(&[FileWithContentToBeTrimmed(
                "foo.nu",
                r#"
                    echo 'foo'
                "#,
            )]);

        let actual = nu!(cwd: dirs.test(), "
            source-env lol/lol.nu;
            $env.RETURN
        ");

        assert_eq!(actual.out, "true");
    })
}
#[test]
fn nu_check_module_dir() {
    Playground::setup("nu_check_test_26", |dirs, sandbox| {
        sandbox
            .mkdir("lol")
            .with_files(&[FileWithContentToBeTrimmed(
                "lol/mod.nu",
                r#"
                    export module foo.nu
                    export def main [] { 'lol' }
                "#,
            )])
            .with_files(&[FileWithContentToBeTrimmed(
                "lol/foo.nu",
                r#"
                    export def main [] { 'lol foo' }
                "#,
            )]);

        let actual = nu!(cwd: dirs.test(), "nu-check lol");

        assert_eq!(actual.out, "true");
    })
}
