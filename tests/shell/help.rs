use nu_path::{AbsolutePath, AbsolutePathBuf};
use nu_test_support::NATIVE_PATH_ENV_VAR;
use nu_test_support::fs::Stub::FileWithContent;
use nu_test_support::nu_with_std;
use nu_test_support::playground::{Dirs, Playground};
use std::env;
use std::path::PathBuf;

#[cfg(unix)]
fn make_executable(path: &AbsolutePath) {
    use std::os::unix::fs::PermissionsExt;

    let std_path: &std::path::Path = path.as_ref();
    let mut perms = std::fs::metadata(std_path).expect("metadata").permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(std_path, perms).expect("set permissions");
}

#[cfg(not(unix))]
fn make_executable(_path: &AbsolutePath) {}

fn write_external_command(
    sandbox: &mut Playground,
    dirs: &Dirs,
    name: &str,
    unix_body: &str,
    windows_body: &str,
) -> AbsolutePathBuf {
    let file_name = if cfg!(windows) {
        format!("{name}.bat")
    } else {
        name.to_string()
    };

    let body = if cfg!(windows) {
        windows_body
    } else {
        unix_body
    };

    sandbox.with_files(&[FileWithContent(&file_name, body)]);

    let path = dirs.test().join(&file_name);
    make_executable(path.as_path());
    path
}

fn playground_env(dirs: &Dirs) -> Vec<(String, String)> {
    let test_dir = dirs.test().to_string_lossy().into_owned();
    let mut path_entries = vec![PathBuf::from(&test_dir)];

    if let Some(original_path) = env::var_os(NATIVE_PATH_ENV_VAR) {
        path_entries.extend(env::split_paths(&original_path));
    }

    let joined_paths = env::join_paths(path_entries)
        .expect("failed to join PATH entries")
        .to_string_lossy()
        .into_owned();

    vec![
        (NATIVE_PATH_ENV_VAR.to_string(), joined_paths),
        ("PLAYGROUND_BIN".to_string(), test_dir),
    ]
}

#[test]
fn external_help_uses_dash_dash_help_by_default() {
    Playground::setup(
        "external_help_uses_dash_dash_help_by_default",
        |dirs, sandbox| {
            write_external_command(
                sandbox,
                &dirs,
                "foo",
                "#!/bin/sh\nif [ \"$1\" = \"--help\" ]; then\n  echo external help content\n  exit 0\nfi\nexit 1\n",
                "@echo off\r\nif \"%1\"==\"--help\" (\r\n  echo external help content\r\n  exit /b 0\r\n)\r\nexit /b 1\r\n",
            );

            let script = r#"use std/help
help foo"#;

            let actual = nu_with_std!(
                cwd: dirs.test(),
                envs: playground_env(&dirs),
                script
            );

            assert!(actual.err.is_empty());
            assert!(actual.out.contains("external help content"));
        },
    );
}

#[test]
fn helper_extra_pipes_output() {
    Playground::setup("helper_extra_pipes_output", |dirs, sandbox| {
        write_external_command(
            sandbox,
            &dirs,
            "foo",
            "#!/bin/sh\nif [ \"$1\" = \"--help\" ]; then\n  echo external help content\n  exit 0\nfi\nexit 1\n",
            "@echo off\r\nif \"%1\"==\"--help\" (\r\n  echo external help content\r\n  exit /b 0\r\n)\r\nexit /b 1\r\n",
        );

        let script = r#"use std/help
$env.NU_HELPER_EXTRA = [ $nu.current-exe "--stdin" "-c" "str upcase" ]
help foo"#;

        let actual = nu_with_std!(
            cwd: dirs.test(),
            envs: playground_env(&dirs),
            script
        );

        assert!(actual.err.is_empty());
        assert!(actual.out.contains("EXTERNAL HELP CONTENT"));
    });
}

#[test]
fn help_falls_back_to_helper_command() {
    Playground::setup("help_falls_back_to_helper_command", |dirs, sandbox| {
        write_external_command(
            sandbox,
            &dirs,
            "broken",
            "#!/bin/sh\nexit 1\n",
            "@echo off\r\nexit /b 1\r\n",
        );

        let helper_path = write_external_command(
            sandbox,
            &dirs,
            "helper",
            "#!/bin/sh\necho fallback help for \"$1\"\n",
            "@echo off\r\necho fallback help for %1\r\n",
        );

        let script = r#"use std/help
$env.NU_HELPER = [$env.HELPER_PATH]
help broken"#;

        let actual = nu_with_std!(
            cwd: dirs.test(),
            envs: {
                let mut envs = playground_env(&dirs);
                envs.push((
                    "HELPER_PATH".to_string(),
                    helper_path.to_string_lossy().into_owned(),
                ));
                envs
            },
            script
        );

        assert!(actual.err.is_empty());
        assert!(actual.out.contains("fallback help for broken"));
    });
}
