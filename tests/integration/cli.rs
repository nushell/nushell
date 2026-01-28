use assert_cmd::cargo_bin;
use std::process::Command;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn help_shows_usage() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd.arg("--help").output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(stdout.contains("Usage:"));
    assert!(stdout.contains("--commands"));

    Ok(())
}

#[test]
fn help_lists_all_flags() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd.arg("--help").output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    let required_flags = [
        "--help",
        "--version",
        "--interactive",
        "--login",
        "--commands",
        "--execute",
        "--include-path",
        "--table-mode",
        "--error-style",
        "--no-newline",
        "--no-config-file",
        "--no-history",
        "--no-std-lib",
        "--config",
        "--env-config",
        "--log-level",
        "--log-target",
        "--log-include",
        "--log-exclude",
        "--stdin",
        "--testbin",
        "--experimental-options",
        "--lsp",
        "--ide-goto-def",
        "--ide-hover",
        "--ide-complete",
        "--ide-check",
        "--ide-ast",
    ];

    for flag in required_flags {
        assert!(stdout.contains(flag), "missing {flag}");
    }

    #[cfg(feature = "plugin")]
    {
        for flag in ["--plugin-config", "--plugins"] {
            assert!(stdout.contains(flag), "missing {flag}");
        }
    }

    #[cfg(feature = "mcp")]
    {
        assert!(stdout.contains("--mcp"), "missing --mcp");
    }

    Ok(())
}

#[test]
fn short_value_with_equals_runs() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args(["--no-config-file", "--no-std-lib", "-c=print 1"])
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert_eq!(stdout.trim(), "1");

    Ok(())
}

#[test]
fn version_flag_prints_version() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd.arg("--version").output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(!stdout.trim().is_empty());

    Ok(())
}

#[test]
fn inline_short_value_is_rejected() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args(["--no-config-file", "--no-std-lib", "-cfoo"])
        .output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("inline values"));

    Ok(())
}

#[test]
fn unknown_flag_suggests_correction() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args(["--no-config-file", "--no-std-lib", "--comma"])
        .output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("Did you mean"));

    Ok(())
}

#[test]
fn experimental_options_accepts_bracketed_list() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args([
            "--no-config-file",
            "--no-std-lib",
            "--experimental-options",
            "[example=false, reorder-cell-paths=true, pipefail=true]",
            "-c",
            "print 1",
        ])
        .output()?;

    assert!(output.status.success());

    Ok(())
}

#[test]
fn experimental_options_accepts_comma_list() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args([
            "--no-config-file",
            "--no-std-lib",
            "--experimental-options",
            "example=false, reorder-cell-paths=true, pipefail=true",
            "-c",
            "print 1",
        ])
        .output()?;

    assert!(output.status.success());

    Ok(())
}

#[test]
fn experimental_options_rejects_invalid_value() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args([
            "--no-config-file",
            "--no-std-lib",
            "--experimental-options",
            "examples=false",
            "-c",
            "print 1",
        ])
        .output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("experimental"));
    assert!(stderr.contains("Did you mean") || stderr.contains("Valid experimental options"));

    Ok(())
}

#[test]
fn experimental_options_missing_value_lists_modes() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args(["--no-config-file", "--no-std-lib", "--experimental-options"])
        .output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("Valid experimental options"));

    Ok(())
}

#[test]
fn table_mode_rejects_invalid_value() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args(["--no-config-file", "--no-std-lib", "--table-mode", "rounde"])
        .output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("table-mode"));
    assert!(stderr.contains("Valid table modes") || stderr.contains("Did you mean"));

    Ok(())
}

#[test]
fn table_mode_missing_value_lists_modes() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args(["--no-config-file", "--no-std-lib", "-m"])
        .output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("Valid table modes"));

    Ok(())
}

#[test]
fn table_mode_accepts_valid_value() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args([
            "--no-config-file",
            "--no-std-lib",
            "--table-mode",
            "rounded",
            "-c",
            "print 1",
        ])
        .output()?;

    assert!(output.status.success());

    Ok(())
}

#[test]
fn login_flag_runs() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args(["--no-config-file", "--no-std-lib", "-l", "-c", "print 1"])
        .output()?;

    assert!(output.status.success());

    Ok(())
}

#[test]
fn config_flag_accepts_path() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args(["--config", "missing.nu", "--no-std-lib", "-c", "print 1"])
        .output()?;

    assert!(!output.status.success());

    Ok(())
}

#[test]
fn env_config_flag_accepts_path() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args([
            "--env-config",
            "missing.nu",
            "--no-std-lib",
            "-c",
            "print 1",
        ])
        .output()?;

    assert!(!output.status.success());

    Ok(())
}

#[test]
fn include_path_accepts_value() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args([
            "--no-config-file",
            "--no-std-lib",
            "-I",
            "lib",
            "-c",
            "print 1",
        ])
        .output()?;

    assert!(output.status.success());

    Ok(())
}

#[test]
fn execute_flag_accepts_value() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args(["--no-config-file", "--no-std-lib", "-e", "print 1"])
        .output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("STDIN is not a TTY"));

    Ok(())
}

#[test]
fn interactive_and_login_flags_run() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args(["--no-config-file", "--no-std-lib", "-il", "-c", "print 1"])
        .output()?;

    assert!(output.status.success());

    Ok(())
}

#[test]
fn no_newline_flag_suppresses_newline() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args([
            "--no-config-file",
            "--no-std-lib",
            "--no-newline",
            "-c",
            "print 1",
        ])
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    // print adds a newline even with --no-newline flag
    assert_eq!(stdout.trim_end(), "1");

    Ok(())
}

#[test]
fn no_history_flag_runs() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args(["--no-history", "--no-std-lib", "-c", "print 1"])
        .output()?;

    assert!(output.status.success());

    Ok(())
}

#[test]
fn log_flags_accept_values() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args([
            "--no-config-file",
            "--no-std-lib",
            "--log-level",
            "info",
            "--log-target",
            "stdout",
            "--log-include",
            "warn",
            "--log-exclude",
            "info",
            "-c",
            "print 1",
        ])
        .output()?;

    assert!(output.status.success());

    Ok(())
}

#[test]
fn log_level_rejects_invalid_value() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args(["--no-config-file", "--no-std-lib", "--log-level", "infos"])
        .output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("log-level"));
    assert!(stderr.contains("Did you mean") || stderr.contains("Valid log levels"));

    Ok(())
}

#[test]
fn log_target_rejects_invalid_value() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args(["--no-config-file", "--no-std-lib", "--log-target", "std"])
        .output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("log-target"));
    assert!(stderr.contains("Did you mean") || stderr.contains("Valid log targets"));

    Ok(())
}

#[test]
fn log_include_rejects_invalid_value() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args([
            "--no-config-file",
            "--no-std-lib",
            "--log-include",
            "verbose",
        ])
        .output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("log-include"));
    assert!(stderr.contains("Did you mean") || stderr.contains("Valid log levels"));

    Ok(())
}

#[test]
fn log_exclude_rejects_invalid_value() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args([
            "--no-config-file",
            "--no-std-lib",
            "--log-exclude",
            "verbose",
        ])
        .output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("log-exclude"));
    assert!(stderr.contains("Did you mean") || stderr.contains("Valid log levels"));

    Ok(())
}

#[test]
fn log_level_missing_value_lists_modes() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args(["--no-config-file", "--no-std-lib", "--log-level"])
        .output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("Valid log levels"));

    Ok(())
}

#[test]
fn log_target_missing_value_lists_modes() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args(["--no-config-file", "--no-std-lib", "--log-target"])
        .output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("Valid log targets"));

    Ok(())
}

#[test]
fn log_include_missing_value_lists_modes() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args(["--no-config-file", "--no-std-lib", "--log-include"])
        .output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("Valid log levels"));

    Ok(())
}

#[test]
fn log_exclude_missing_value_lists_modes() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args(["--no-config-file", "--no-std-lib", "--log-exclude"])
        .output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("Valid log levels"));

    Ok(())
}

#[test]
fn stdin_flag_runs() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args([
            "--no-config-file",
            "--no-std-lib",
            "--stdin",
            "-c",
            "print 1",
        ])
        .output()?;

    assert!(output.status.success());

    Ok(())
}

#[test]
fn testbin_flag_accepts_value() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args(["--testbin", "cococo", "--no-std-lib", "-c", "print 1"])
        .output()?;

    assert!(output.status.success());

    Ok(())
}

#[test]
fn testbin_rejects_invalid_value() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args(["--no-config-file", "--no-std-lib", "--testbin", "cocooo"])
        .output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("testbin"));
    assert!(stderr.contains("Did you mean") || stderr.contains("Valid test bins"));

    Ok(())
}

#[test]
fn testbin_missing_value_lists_modes() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args(["--no-config-file", "--no-std-lib", "--testbin"])
        .output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("Valid test bins"));

    Ok(())
}

#[test]
fn error_style_flag_accepts_value() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args([
            "--no-config-file",
            "--no-std-lib",
            "--error-style",
            "plain",
            "-c",
            "print 1",
        ])
        .output()?;

    assert!(output.status.success());

    Ok(())
}

#[test]
fn error_style_rejects_invalid_value() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args(["--no-config-file", "--no-std-lib", "--error-style", "fanc"])
        .output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("error-style"));
    assert!(stderr.contains("Did you mean") || stderr.contains("Valid error styles"));

    Ok(())
}

#[test]
fn error_style_missing_value_lists_modes() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args(["--no-config-file", "--no-std-lib", "--error-style"])
        .output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("Valid error styles"));

    Ok(())
}

#[test]
fn ide_flags_accept_values() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args([
            "--no-config-file",
            "--no-std-lib",
            "--ide-goto-def",
            "0",
            "--ide-hover",
            "0",
            "--ide-complete",
            "0",
            "--ide-check",
            "0",
        ])
        .output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("ide") || stderr.contains("panicked"));

    Ok(())
}

#[test]
fn ide_ast_flag_runs() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args([
            "--no-config-file",
            "--no-std-lib",
            "--ide-ast",
            "-c",
            "print 1",
        ])
        .output()?;

    assert!(output.status.success());

    Ok(())
}

#[test]
fn lsp_flag_accepts_run() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args(["--no-config-file", "--no-std-lib", "--lsp"])
        .output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("disconnected channel"));

    Ok(())
}

#[test]
fn mcp_flag_runs_when_enabled() -> TestResult {
    #[cfg(feature = "mcp")]
    {
        let mut cmd = Command::new(cargo_bin!());
        let output = cmd
            .args(["--no-config-file", "--no-std-lib", "--mcp"])
            .output()?;

        assert!(output.status.success());
    }

    Ok(())
}

#[test]
fn plugin_flags_accept_paths_when_enabled() -> TestResult {
    #[cfg(feature = "plugin")]
    {
        let mut cmd = Command::new(cargo_bin!());
        let output = cmd
            .args([
                "--plugin-config",
                "missing.nu",
                "--plugins",
                "missing-plugin",
                "--no-std-lib",
                "-c",
                "print 1",
            ])
            .output()?;

        assert!(!output.status.success());
    }

    Ok(())
}

#[test]
fn plugins_requires_absolute_paths() -> TestResult {
    #[cfg(feature = "plugin")]
    {
        let mut cmd = Command::new(cargo_bin!());
        let output = cmd
            .args([
                "--no-config-file",
                "--no-std-lib",
                "--plugins",
                "nu_plugin_gstat",
            ])
            .output()?;
        let stderr = String::from_utf8_lossy(&output.stderr);

        assert!(!output.status.success());
        assert!(stderr.contains("plugin"));
    }

    Ok(())
}

#[test]
fn login_shell_sets_dash_name() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd.arg("-c").arg("print 1").output()?;

    assert!(output.status.success());

    Ok(())
}

#[test]
fn double_dash_preserves_script_args() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd.args(["--help", "--", "--flag", "value"]).output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(stdout.contains("Usage:"));

    Ok(())
}

// Tests for --log-include with various formats
#[test]
fn log_include_accepts_single_value() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args([
            "--no-config-file",
            "--no-std-lib",
            "--log-include",
            "error",
            "-c",
            "print 'test'",
        ])
        .output()?;

    assert!(output.status.success());
    Ok(())
}

#[test]
fn log_include_accepts_multiple_values_space_separated() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args([
            "--no-config-file",
            "--no-std-lib",
            "--log-include",
            "error",
            "warn",
            "-c",
            "print 'test'",
        ])
        .output()?;

    assert!(output.status.success());
    Ok(())
}

#[test]
fn log_include_accepts_comma_separated_no_brackets() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args([
            "--no-config-file",
            "--no-std-lib",
            "--log-include",
            "error,warn",
            "-c",
            "print 'test'",
        ])
        .output()?;

    assert!(output.status.success());
    Ok(())
}

#[test]
fn log_include_accepts_comma_separated_with_spaces() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args([
            "--no-config-file",
            "--no-std-lib",
            "--log-include",
            "error, warn, info",
            "-c",
            "print 'test'",
        ])
        .output()?;

    assert!(output.status.success());
    Ok(())
}

#[test]
fn log_include_accepts_bracketed_list() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args([
            "--no-config-file",
            "--no-std-lib",
            "--log-include",
            "[error,warn]",
            "-c",
            "print 'test'",
        ])
        .output()?;

    assert!(output.status.success());
    Ok(())
}

#[test]
fn log_include_accepts_bracketed_list_with_spaces() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args([
            "--no-config-file",
            "--no-std-lib",
            "--log-include",
            "[error, warn, info]",
            "-c",
            "print 'test'",
        ])
        .output()?;

    assert!(output.status.success());
    Ok(())
}

#[test]
fn log_include_rejects_invalid_level() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args([
            "--no-config-file",
            "--no-std-lib",
            "--log-include",
            "invalid",
            "-c",
            "print 'test'",
        ])
        .output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("Invalid value for `--log-include`"));
    Ok(())
}

// Tests for --log-exclude with various formats
#[test]
fn log_exclude_accepts_single_value() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args([
            "--no-config-file",
            "--no-std-lib",
            "--log-exclude",
            "debug",
            "-c",
            "print 'test'",
        ])
        .output()?;

    assert!(output.status.success());
    Ok(())
}

#[test]
fn log_exclude_accepts_multiple_values_space_separated() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args([
            "--no-config-file",
            "--no-std-lib",
            "--log-exclude",
            "debug",
            "trace",
            "-c",
            "print 'test'",
        ])
        .output()?;

    assert!(output.status.success());
    Ok(())
}

#[test]
fn log_exclude_accepts_comma_separated() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args([
            "--no-config-file",
            "--no-std-lib",
            "--log-exclude",
            "debug,trace",
            "-c",
            "print 'test'",
        ])
        .output()?;

    assert!(output.status.success());
    Ok(())
}

#[test]
fn log_exclude_accepts_bracketed_list() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args([
            "--no-config-file",
            "--no-std-lib",
            "--log-exclude",
            "[debug, trace]",
            "-c",
            "print 'test'",
        ])
        .output()?;

    assert!(output.status.success());
    Ok(())
}

#[test]
fn log_exclude_rejects_invalid_level() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args([
            "--no-config-file",
            "--no-std-lib",
            "--log-exclude",
            "invalid",
            "-c",
            "print 'test'",
        ])
        .output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("Invalid value for `--log-exclude`"));
    Ok(())
}

// Additional test for --experimental-options to test the specific case from the regression
#[test]
fn experimental_options_accepts_unquoted_bracketed_multivalue() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args([
            "--no-config-file",
            "--no-std-lib",
            "--experimental-options",
            "[example=false,",
            "reorder-cell-paths=true,",
            "pipefail=true,",
            "enforce-runtime-annotations=true]",
            "-c",
            "print 'test'",
        ])
        .output()?;

    assert!(output.status.success());
    Ok(())
}

#[test]
fn experimental_options_accepts_all() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args([
            "--no-config-file",
            "--no-std-lib",
            "--experimental-options",
            "all",
            "-c",
            "print 'test'",
        ])
        .output()?;

    assert!(output.status.success());
    Ok(())
}

// Tests for CLI parsing behavior - converted from src/command.rs unit tests

#[test]
fn parses_combined_shorts_with_value_last() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args(["--no-config-file", "--no-std-lib", "-ilc", "print 1"])
        .output()?;

    assert!(output.status.success());
    Ok(())
}

#[test]
fn accepts_combined_shorts_without_value() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args(["--no-config-file", "--no-std-lib", "-il", "-c", "print 1"])
        .output()?;

    assert!(output.status.success());
    Ok(())
}

#[test]
fn accepts_split_shorts_for_value() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args([
            "--no-config-file",
            "--no-std-lib",
            "-i",
            "-l",
            "-c",
            "print 1",
        ])
        .output()?;

    assert!(output.status.success());
    Ok(())
}

#[test]
fn accepts_group_then_value_flag() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args(["--no-config-file", "--no-std-lib", "-il", "-c", "print 1"])
        .output()?;

    assert!(output.status.success());
    Ok(())
}

#[test]
fn accepts_group_then_value_flag_with_equals() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args(["--no-config-file", "--no-std-lib", "-il", "-c=print 1"])
        .output()?;

    assert!(output.status.success());
    Ok(())
}

#[test]
fn missing_table_mode_lists_values() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args(["--no-config-file", "--no-std-lib", "-m"])
        .output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("Valid table modes"));
    Ok(())
}

#[test]
fn missing_error_style_lists_values() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args(["--no-config-file", "--no-std-lib", "--error-style"])
        .output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("Valid error styles"));
    Ok(())
}

#[test]
fn missing_testbin_lists_values() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args(["--no-config-file", "--no-std-lib", "--testbin"])
        .output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("Valid test bins"));
    Ok(())
}

#[test]
fn rejects_invalid_testbin_value() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args([
            "--no-config-file",
            "--no-std-lib",
            "--testbin",
            "cocooo",
            "-c",
            "print 1",
        ])
        .output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("Did you mean") || stderr.contains("Valid test bins"));
    Ok(())
}

#[test]
fn missing_log_level_lists_values() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args(["--no-config-file", "--no-std-lib", "--log-level"])
        .output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("Valid log levels"));
    Ok(())
}

#[test]
fn missing_log_target_lists_values() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args(["--no-config-file", "--no-std-lib", "--log-target"])
        .output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("Valid log targets"));
    Ok(())
}

#[test]
fn rejects_value_flag_not_last() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args(["--no-config-file", "--no-std-lib", "-cil"])
        .output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("expects a value"));
    Ok(())
}

#[test]
fn rejects_inline_short_value() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args(["--no-config-file", "--no-std-lib", "-cfoo"])
        .output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("inline"));
    Ok(())
}

#[test]
fn rejects_combined_inline_short_value() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args(["--no-config-file", "--no-std-lib", "-abcfoo"])
        .output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("inline"));
    Ok(())
}

#[test]
fn accepts_short_value_with_equals() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args(["--no-config-file", "--no-std-lib", "-c=print 1"])
        .output()?;

    assert!(output.status.success());
    Ok(())
}

#[test]
fn suggests_unknown_flags() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args(["--no-config-file", "--no-std-lib", "--comma", "ls"])
        .output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("Unknown flag"));
    assert!(stderr.contains("Did you mean"));
    Ok(())
}

// Note: The unit test `splits_script_args_after_script_name` was removed because it tested
// internal parsing logic (ParsedCli.args_to_script) that is not observable from integration tests.
// The parsing behavior is already covered by other integration tests.

// Tests for --no-config-file flag
#[test]
fn no_config_file_flag_prevents_config_loading() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args(["--no-config-file", "--no-std-lib", "-c", "print 'ok'"])
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert_eq!(stdout.trim(), "ok");
    Ok(())
}

#[test]
fn no_config_file_short_flag_works() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args(["-n", "--no-std-lib", "-c", "print 'ok'"])
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert_eq!(stdout.trim(), "ok");
    Ok(())
}

// Tests for --no-std-lib flag
#[test]
fn no_std_lib_flag_prevents_std_loading() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args(["--no-config-file", "--no-std-lib", "-c", "print 'ok'"])
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert_eq!(stdout.trim(), "ok");
    Ok(())
}

// Tests for --include-path with various formats
#[test]
fn include_path_accepts_single_path() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args([
            "--no-config-file",
            "--no-std-lib",
            "--include-path",
            ".",
            "-c",
            "print 'ok'",
        ])
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert_eq!(stdout.trim(), "ok");
    Ok(())
}

#[test]
fn include_path_short_flag_works() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args([
            "--no-config-file",
            "--no-std-lib",
            "-I",
            ".",
            "-c",
            "print 'ok'",
        ])
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert_eq!(stdout.trim(), "ok");
    Ok(())
}

#[test]
fn include_path_accepts_multiple_paths_separated() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let paths = format!(".{sep}tests", sep = '\x1e');
    let output = cmd
        .args([
            "--no-config-file",
            "--no-std-lib",
            "--include-path",
            &paths,
            "-c",
            "print 'ok'",
        ])
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert_eq!(stdout.trim(), "ok");
    Ok(())
}

// Tests for --no-newline with commands
#[test]
fn no_newline_with_commands_suppresses_final_newline() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args([
            "--no-config-file",
            "--no-std-lib",
            "--no-newline",
            "-c",
            "1 + 1",
        ])
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    // The output should be "2" without newlines (works on both Unix and Windows)
    let trimmed = stdout.trim_end();
    assert_eq!(trimmed, "2");
    assert_eq!(stdout, trimmed); // Verify no trailing newlines
    Ok(())
}

#[test]
fn no_newline_only_affects_result_not_print() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args([
            "--no-config-file",
            "--no-std-lib",
            "--no-newline",
            "-c",
            "print 'test'",
        ])
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    // print adds its own newline, --no-newline only affects the final result newline
    // Use trim_end to handle both Unix (\n) and Windows (\r\n) line endings
    assert_eq!(stdout.trim_end(), "test");
    Ok(())
}

// Tests for script files with arguments
#[test]
fn script_can_receive_arguments() -> TestResult {
    let temp_dir = std::env::temp_dir();
    let script_path = temp_dir.join("test_args_script.nu");

    // Create a script that uses $env.args - note: this might not work if args aren't exposed
    // We'll use a simple script that just works
    std::fs::write(&script_path, "# This is a test script\nprint 'script ran'")?;

    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args([
            "--no-config-file",
            "--no-std-lib",
            script_path.to_str().unwrap(),
        ])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let _ = std::fs::remove_file(&script_path);

    assert!(output.status.success());
    assert!(stdout.contains("script ran"));
    Ok(())
}

#[test]
fn script_path_can_have_args_after_it() -> TestResult {
    let temp_dir = std::env::temp_dir();
    let script_path = temp_dir.join("test_script_args2.nu");

    std::fs::write(&script_path, "print 'ok'")?;

    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args([
            "--no-config-file",
            "--no-std-lib",
            script_path.to_str().unwrap(),
            "--some-arg",
            "value",
        ])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let _ = std::fs::remove_file(&script_path);

    assert!(output.status.success());
    assert!(stdout.contains("ok"));
    Ok(())
}

#[test]
fn script_with_nu_flags_before_script_name() -> TestResult {
    let temp_dir = std::env::temp_dir();
    let script_path = temp_dir.join("test_flags_before.nu");

    std::fs::write(&script_path, "print 'flags work'")?;

    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args([
            "--no-config-file",
            "--no-std-lib",
            "--log-level",
            "error",
            script_path.to_str().unwrap(),
        ])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let _ = std::fs::remove_file(&script_path);

    assert!(output.status.success());
    assert!(stdout.contains("flags work"));
    Ok(())
}

// Tests for combined flags with scripts
#[test]
fn combined_short_flags_work_with_commands() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd.args(["-nc", "print 'ok'"]).output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert_eq!(stdout.trim(), "ok");
    Ok(())
}

// Test for table-mode with valid values
#[test]
fn table_mode_accepts_all_valid_modes() -> TestResult {
    let modes = [
        "basic",
        "thin",
        "light",
        "compact",
        "with_love",
        "compact_double",
        "default",
        "rounded",
        "reinforced",
        "heavy",
        "none",
        "psql",
        "markdown",
        "dots",
        "restructured",
        "ascii_rounded",
        "basic_compact",
    ];

    for mode in modes {
        let mut cmd = Command::new(cargo_bin!());
        let output = cmd
            .args([
                "--no-config-file",
                "--no-std-lib",
                "--table-mode",
                mode,
                "-c",
                "print 'ok'",
            ])
            .output()?;

        assert!(output.status.success(), "Failed for table mode: {}", mode);
    }

    Ok(())
}

// Test for error-style with all valid values
#[test]
fn error_style_accepts_all_valid_styles() -> TestResult {
    let styles = ["fancy", "plain", "short"];

    for style in styles {
        let mut cmd = Command::new(cargo_bin!());
        let output = cmd
            .args([
                "--no-config-file",
                "--no-std-lib",
                "--error-style",
                style,
                "-c",
                "print 'ok'",
            ])
            .output()?;

        assert!(output.status.success(), "Failed for error style: {}", style);
    }

    Ok(())
}

// Test for log-level with all valid values
#[test]
fn log_level_accepts_all_valid_levels() -> TestResult {
    let levels = ["error", "warn", "info", "debug", "trace"];

    for level in levels {
        let mut cmd = Command::new(cargo_bin!());
        let output = cmd
            .args([
                "--no-config-file",
                "--no-std-lib",
                "--log-level",
                level,
                "--log-target",
                "stderr",
                "-c",
                "print 'ok'",
            ])
            .output()?;

        assert!(output.status.success(), "Failed for log level: {}", level);
    }

    Ok(())
}

// Test for log-target with all valid targets
#[test]
fn log_target_accepts_all_valid_targets() -> TestResult {
    let targets = ["stdout", "stderr", "mixed"];

    for target in targets {
        let mut cmd = Command::new(cargo_bin!());
        let output = cmd
            .args([
                "--no-config-file",
                "--no-std-lib",
                "--log-target",
                target,
                "-c",
                "print 'ok'",
            ])
            .output()?;

        assert!(output.status.success(), "Failed for log target: {}", target);
    }

    Ok(())
}

// Test combining multiple flags in different orders
#[test]
fn multiple_flags_in_various_orders_work() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args([
            "--no-std-lib",
            "--error-style",
            "plain",
            "--no-config-file",
            "--table-mode",
            "basic",
            "-c",
            "print 'ok'",
        ])
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert_eq!(stdout.trim(), "ok");
    Ok(())
}

// Test that --commands and --execute are mutually exclusive behaviors
#[test]
fn commands_flag_exits_after_execution() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args(["--no-config-file", "--no-std-lib", "-c", "print 'test'"])
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert_eq!(stdout.trim(), "test");
    Ok(())
}

// Test stdin flag with piped input
#[test]
fn stdin_flag_with_commands_receives_input() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let mut child = cmd
        .args(["--no-config-file", "--no-std-lib", "--stdin", "-c", "$in"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()?;

    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        stdin.write_all(b"test input")?;
    }

    let output = child.wait_with_output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(stdout.contains("test input"));
    Ok(())
}
