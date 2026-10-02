use nu_test_support::{fs::Stub::FileWithContent, prelude::*};

#[test]
#[deps(NU)]
fn pickled_script_runs_without_its_sources() -> Result {
    Playground::setup("pickled_script", |dirs, sandbox| -> Result {
        sandbox.with_files(&[FileWithContent(
            "script.nu",
            "
                use lib/helper.nu
                source lib/extra.nu

                # Doubles and increments
                def main [x: int] { print (helper add-one (extra-double $x)) }
            ",
        )]);
        sandbox.within("lib").with_files(&[
            FileWithContent("helper.nu", "export def add-one [x: int] { $x + 1 }"),
            FileWithContent("extra.nu", "def extra-double [x: int] { $x * 2 }"),
        ]);

        let result: CompleteResult = test()
            .cwd(dirs.test())
            .run("nu -n -c 'pickle script.nu' | complete")?;
        assert_eq!(result.exit_code, 0);
        assert_eq!(result.stderr, "");

        // The pickle holds the compiled script, module and sourced file, and it relinks against
        // a differently started engine.
        std::fs::remove_file(dirs.test().join("script.nu"))?;
        std::fs::remove_dir_all(dirs.test().join("lib"))?;
        let result: CompleteResult = test()
            .cwd(dirs.test())
            .run("nu --no-std-lib -n script.nupkl 20 | complete")?;
        assert_eq!(result.exit_code, 0);
        assert_eq!(result.stdout, "41\n");

        let help: String = test()
            .cwd(dirs.test())
            .run("nu -n script.nupkl --help | ansi strip")?;
        assert_contains("Doubles and increments", &help);
        assert_contains("script.nupkl <x>", help);
        Ok(())
    })
}

#[test]
#[deps(NU)]
fn pickle_carries_what_the_shell_already_loaded() -> Result {
    Playground::setup("pickle_loaded", |dirs, sandbox| -> Result {
        sandbox.with_files(&[
            FileWithContent(
                "script.nu",
                "use lib.nu\ndef main [] { print $'(lib hi) ran' }",
            ),
            FileWithContent(
                "lib.nu",
                "const GREETING = 'hi'\nexport def hi [] { $GREETING }",
            ),
            FileWithContent("uses_shell.nu", "print (lib hi)"),
            FileWithContent("config.nu", "use lib.nu"),
            FileWithContent("env.nu", ""),
        ]);

        // This shell has parsed the script and everything it uses already.
        let result: CompleteResult = test()
            .cwd(dirs.test())
            .run("nu -n -c 'source script.nu; pickle script.nu' | complete")?;
        assert_eq!(result.exit_code, 0);
        let config = "nu --config config.nu --env-config env.nu";
        for run in ["nu -n", config] {
            let result: CompleteResult = test()
                .cwd(dirs.test())
                .run(format!("{run} script.nupkl | complete"))?;
            assert_eq!(result.stdout, "hi ran\n");
        }

        // `lib hi` comes from the shell, and its body reads a constant only `lib.nu` sees.
        let result: CompleteResult = test()
            .cwd(dirs.test())
            .run("nu -n -c 'use lib.nu; pickle uses_shell.nu' | complete")?;
        assert_eq!(result.exit_code, 0);
        let result: CompleteResult = test()
            .cwd(dirs.test())
            .run(format!("{config} uses_shell.nupkl | complete"))?;
        assert_eq!(result.stdout, "hi\n");
        Ok(())
    })
}

// Looking for a pickle header mustn't take bytes from a script read from a pipe.
#[cfg(unix)]
#[test]
#[deps(NU)]
fn script_from_a_pipe_runs_whole() -> Result {
    let result: CompleteResult =
        test().run("'print one; print two' | nu -n /dev/stdin | complete")?;
    assert_eq!(result.stdout, "one\ntwo\n");
    Ok(())
}

#[test]
#[deps(NU)]
fn source_or_use_of_a_pickle_says_what_it_is() -> Result {
    Playground::setup("pickle_source", |dirs, sandbox| -> Result {
        sandbox.with_files(&[FileWithContent("lib.nu", "export def hi [] { 'hi' }")]);
        let result: CompleteResult = test()
            .cwd(dirs.test())
            .run("nu -n -c 'pickle lib.nu' | complete")?;
        assert_eq!(result.exit_code, 0);

        for code in ["source lib.nupkl", "use lib.nupkl"] {
            let err = test().cwd(dirs.test()).run(code).expect_parse_error()?;
            assert!(
                matches!(&err, ParseError::LabeledErrorWithHelp { error, .. }
                    if error == "Can't read a pickle as source code"),
                "`{code}` failed with {err:?}"
            );
        }
        Ok(())
    })
}

#[test]
fn pickle_refuses_a_missing_script() -> Result {
    test()
        .run("pickle does-not-exist.nu")
        .expect_error_code_eq("nu::shell::io::file_not_found")
}

#[test]
#[deps(NU)]
fn pickle_writes_the_file_asked_for_but_only_over_a_pickle() -> Result {
    Playground::setup("pickle_output", |dirs, sandbox| -> Result {
        sandbox.with_files(&[
            FileWithContent("script.nu", "print hi"),
            FileWithContent("notes.txt", "keep me"),
        ]);

        // The second time replaces the first pickle.
        for _ in 0..2 {
            let result: CompleteResult = test()
                .cwd(dirs.test())
                .run("nu -n -c 'pickle script.nu -o custom.nupkl' | complete")?;
            assert_eq!(result.exit_code, 0);
        }
        let result: CompleteResult = test()
            .cwd(dirs.test())
            .run("nu -n custom.nupkl | complete")?;
        assert_eq!(result.stdout, "hi\n");

        // Any other file is kept, including the script itself.
        for (output, content) in [("script.nu", "print hi"), ("notes.txt", "keep me")] {
            let err = test()
                .cwd(dirs.test())
                .run_with_data("pickle script.nu -o $in", output)
                .expect_shell_error()?;
            assert_contains("isn't a pickle", err.generic_msg()?);
            assert_eq!(std::fs::read_to_string(dirs.test().join(output))?, content);
        }
        Ok(())
    })
}

#[test]
#[deps(NU)]
fn pickle_warns_that_it_compiles_the_shells_aliases() -> Result {
    Playground::setup("pickle_aliases", |dirs, sandbox| -> Result {
        sandbox.with_files(&[FileWithContent("script.nu", "print (ls)")]);

        let result: CompleteResult = test()
            .cwd(dirs.test())
            .run("nu -n -c 'alias ls = echo aliased; pickle script.nu | get warnings | to nuon' | complete")?;
        assert_eq!(result.exit_code, 0);
        assert_contains("`ls` is an alias in this shell", result.stdout);
        let result: CompleteResult = test()
            .cwd(dirs.test())
            .run("nu -n script.nupkl | complete")?;
        assert_eq!(result.stdout, "aliased\n");

        // Without the alias there's nothing to warn about.
        let result: CompleteResult = test()
            .cwd(dirs.test())
            .run("nu -n -c 'pickle script.nu | get warnings | to nuon' | complete")?;
        assert_eq!(result.stdout, "[]\n");
        Ok(())
    })
}

#[test]
#[deps(NU)]
fn pickle_runs_as_a_script_whatever_overlay_is_active() -> Result {
    Playground::setup("pickle_overlay", |dirs, sandbox| -> Result {
        sandbox.with_files(&[FileWithContent("script.nu", "def main [] { print ran }")]);

        let result: CompleteResult = test()
            .cwd(dirs.test())
            .run("nu -n -c 'module spam {}; overlay use spam; pickle script.nu' | complete")?;
        assert_eq!(result.stderr, "");
        let result: CompleteResult = test()
            .cwd(dirs.test())
            .run("nu -n script.nupkl | complete")?;
        assert_eq!(result.stdout, "ran\n");
        Ok(())
    })
}

#[test]
#[deps(NU)]
fn pickle_runs_with_the_experimental_options_it_was_made_with() -> Result {
    Playground::setup("pickle_options", |dirs, sandbox| -> Result {
        // `dc-glob` is off by default.
        sandbox.with_files(&[FileWithContent(
            "script.nu",
            "print (debug experimental-options | where identifier == dc-glob | get 0.enabled)",
        )]);

        let result: CompleteResult = test().cwd(dirs.test()).run(
            "nu -n '--experimental-options=[dc-glob]' -c 'pickle script.nu -o on.nupkl' | complete",
        )?;
        assert_eq!(result.exit_code, 0);
        let result: CompleteResult = test().cwd(dirs.test()).run("nu -n on.nupkl | complete")?;
        assert_eq!(result.stdout, "true\n");

        // An option that was off when the pickle was made stays off, and asking for it says so.
        let result: CompleteResult = test()
            .cwd(dirs.test())
            .run("nu -n -c 'pickle script.nu -o off.nupkl' | complete")?;
        assert_eq!(result.exit_code, 0);
        let result: CompleteResult = test()
            .cwd(dirs.test())
            .run("nu -n '--experimental-options=[dc-glob]' off.nupkl | complete")?;
        assert_eq!(result.stdout, "false\n");
        assert_contains("doesn't apply to a pickle", result.stderr);
        Ok(())
    })
}

#[test]
#[deps(NU)]
fn pickled_script_is_portable() -> Result {
    Playground::setup("pickle_portable", |dirs, sandbox| -> Result {
        sandbox.mkdir("project/lib").mkdir("elsewhere");
        sandbox.within("project").with_files(&[FileWithContent(
            "script.nu",
            "
                use lib/tools.nu
                source lib/helpers.nu
                source-env lib/vars.nu

                def main [--fail] {
                    print (tools shout (greet $env.VARS_LOADED))
                    print $env.TOOLS_PWD
                    print $env.INNER_FILE
                    print $env.HELPERS_FILE
                    print $env.VARS_FILE
                    if $fail { error make {msg: 'failed on purpose'} }
                }
            ",
        )]);
        sandbox.within("project/lib").with_files(&[
            FileWithContent(
                "tools.nu",
                "
                    export-env { export use inner.nu; $env.TOOLS_PWD = $env.FILE_PWD }
                    export def shout [s: string] { $s | str uppercase }
                ",
            ),
            FileWithContent(
                "inner.nu",
                "export-env { $env.INNER_FILE = $env.CURRENT_FILE }",
            ),
            FileWithContent(
                "helpers.nu",
                "def greet [s: string] { $'hello ($s)' }; $env.HELPERS_FILE = $env.CURRENT_FILE",
            ),
            FileWithContent(
                "vars.nu",
                "$env.VARS_LOADED = 'vars'; $env.VARS_FILE = $env.CURRENT_FILE",
            ),
        ]);

        let project = dirs.test().join("project");
        let result: CompleteResult = test()
            .cwd(&project)
            .run("nu -n -c 'pickle script.nu' | complete")?;
        assert_eq!(result.exit_code, 0);

        // Only the pickle moves, and the project it came from is gone.
        let elsewhere = dirs.test().join("elsewhere");
        std::fs::rename(project.join("script.nupkl"), elsewhere.join("script.nupkl"))?;
        std::fs::remove_dir_all(&project)?;

        let result: CompleteResult = test()
            .cwd(&elsewhere)
            .run("nu --no-std-lib -n script.nupkl | complete")?;
        assert_eq!(result.stderr, "");
        // The `$env.FILE_PWD` and `$env.CURRENT_FILE` of the modules and sourced files point
        // next to the pickle now.
        let lib = elsewhere.join("lib");
        assert_eq!(
            result.stdout,
            format!(
                "HELLO VARS\n{}\n{}\n{}\n{}\n",
                lib.display(),
                lib.join("inner.nu").display(),
                lib.join("helpers.nu").display(),
                lib.join("vars.nu").display()
            )
        );

        // Errors still show the source, which the pickle carries.
        let result: CompleteResult = test()
            .cwd(&elsewhere)
            .run("nu -n script.nupkl --fail | complete")?;
        assert_ne!(result.exit_code, 0);
        assert_contains("failed on purpose", &result.stderr);
        assert_contains("if $fail { error make", result.stderr);
        Ok(())
    })
}

#[test]
#[deps(NU)]
fn pickled_parse_time_values_come_from_where_it_runs() -> Result {
    Playground::setup("pickle_parse_time", |dirs, sandbox| -> Result {
        sandbox.mkdir("project/lib").mkdir("elsewhere");
        sandbox.within("project").with_files(&[FileWithContent(
            "script.nu",
            "
                use lib/ids.nu
                use lib/ids.nu [PID]
                const pid = $nu.pid
                const here = path self

                # `$nu.pid` differs between the nu that pickles and the one that runs.
                def main [--default: int = $pid] {
                    print ([$pid $default $PID $ids.PID] | all { $in == $nu.pid })
                    print ($here | path dirname | path basename)
                }
            ",
        )]);
        sandbox
            .within("project/lib")
            .with_files(&[FileWithContent("ids.nu", "export const PID = $nu.pid")]);

        // The shell's `$pid` doesn't take the place of the script's.
        let project = dirs.test().join("project");
        let result: CompleteResult = test()
            .cwd(&project)
            .run("nu -n -c 'const pid = 0; pickle script.nu' | complete")?;
        assert_eq!(result.exit_code, 0);
        let elsewhere = dirs.test().join("elsewhere");
        std::fs::rename(project.join("script.nupkl"), elsewhere.join("script.nupkl"))?;

        let result: CompleteResult = test()
            .cwd(&elsewhere)
            .run("nu -n script.nupkl | complete")?;
        assert_eq!(result.stderr, "");
        assert_eq!(result.stdout, "true\nelsewhere\n");
        Ok(())
    })
}

#[test]
#[deps(NU)]
fn depickle_writes_the_sources_back() -> Result {
    Playground::setup("depickle", |dirs, sandbox| -> Result {
        sandbox.mkdir("project/lib").mkdir("shared").mkdir("out");
        let script = "use lib/tools.nu\nuse ../shared/common.nu\nprint (tools hi) (common hi)\n";
        let tools = "export def hi [] { 'tools' }";
        let common = "export def hi [] { 'common' }";
        sandbox
            .within("project")
            .with_files(&[FileWithContent("script.nu", script)]);
        sandbox
            .within("project/lib")
            .with_files(&[FileWithContent("tools.nu", tools)]);
        sandbox
            .within("shared")
            .with_files(&[FileWithContent("common.nu", common)]);

        let result: CompleteResult = test()
            .cwd(dirs.test().join("project"))
            .run("nu -n -c 'pickle script.nu' | complete")?;
        assert_eq!(result.exit_code, 0);

        let out = dirs.test().join("out");
        let result: CompleteResult = test()
            .cwd(&out)
            .run("nu -n -c 'depickle ../project/script.nupkl' | complete")?;
        assert_eq!(result.exit_code, 0);
        assert_eq!(std::fs::read_to_string(out.join("script.nu"))?, script);
        assert_eq!(std::fs::read_to_string(out.join("lib/tools.nu"))?, tools);
        // `..` stays inside the output directory.
        assert_eq!(
            std::fs::read_to_string(out.join("_up/shared/common.nu"))?,
            common
        );

        let result: CompleteResult = test()
            .cwd(&out)
            .run("nu -n -c 'depickle ../project/script.nupkl' | complete")?;
        assert_ne!(result.exit_code, 0);
        assert_contains("already exists", result.stderr);
        let result: CompleteResult = test()
            .cwd(&out)
            .run("nu -n -c 'depickle ../project/script.nupkl --force' | complete")?;
        assert_eq!(result.exit_code, 0);
        Ok(())
    })
}

#[test]
#[deps(NU)]
fn debug_pickle_says_what_keeps_it_from_loading() -> Result {
    Playground::setup("debug_pickle", |dirs, sandbox| -> Result {
        sandbox.with_files(&[FileWithContent("script.nu", "def main [] { print hi }")]);
        let result: CompleteResult = test()
            .cwd(dirs.test())
            .run("nu -n -c 'pickle script.nu' | complete")?;
        assert_eq!(result.exit_code, 0);

        let result: CompleteResult = test()
            .cwd(dirs.test())
            .run("nu -n -c 'debug pickle script.nupkl | [$in.loads ...$in.commands.name] | to nuon' | complete")?;
        assert_eq!(result.stdout.trim(), "[true, main]");

        // A pickle from another version.
        let version = env!("CARGO_PKG_VERSION");
        let pickle = std::fs::read(dirs.test().join("script.nupkl"))?;
        let at = pickle
            .windows(version.len())
            .position(|window| window == version.as_bytes())
            .expect("the header names the version");
        let mut other = pickle.clone();
        other[at] = if other[at] == b'9' { b'8' } else { b'9' };
        std::fs::write(dirs.test().join("other.nupkl"), &other)?;
        let other_version = String::from_utf8_lossy(&other[at..at + version.len()]).into_owned();

        let result: CompleteResult = test()
            .cwd(dirs.test())
            .run("nu -n -c 'debug pickle other.nupkl | [$in.loads ...$in.problems] | to nuon' | complete")?;
        assert_eq!(
            result.stdout.trim(),
            format!("[false, \"expected version {version} and got {other_version}\"]")
        );

        let result: CompleteResult = test()
            .cwd(dirs.test())
            .run("nu -n other.nupkl | complete")?;
        assert_ne!(result.exit_code, 0);
        assert_contains(
            format!("expected version {version} and got {other_version}"),
            result.stderr,
        );
        Ok(())
    })
}
