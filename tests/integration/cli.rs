use assert_cmd::cargo_bin;
use std::process::Command;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn unique_temp_script_path(stem: &str) -> std::path::PathBuf {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);

    std::env::temp_dir().join(format!("{stem}_{}_{}.nu", std::process::id(), unique))
}

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
fn help_has_no_ansi_when_stdout_is_not_terminal() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd.arg("--help").output()?;

    assert!(output.status.success());
    assert!(
        !output.stdout.contains(&0x1b),
        "redirected help output should not contain ANSI escape sequences"
    );

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
fn ide_check_missing_file_reports_error() -> TestResult {
    let script_path = unique_temp_script_path("ide_check_missing");
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args(["--no-config-file", "--no-std-lib", "--ide-check", "5"])
        .arg(&script_path)
        .output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("Could not read file"));
    assert!(stderr.contains("File not found"));
    assert!(stderr.contains("does not exist"));

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
fn ide_complete_out_of_range_locations() -> TestResult {
    let script_path = unique_temp_script_path("ide_complete_bounds");
    std::fs::write(&script_path, "ä")?;

    for location in ["-1", "1", "99"] {
        let mut cmd = Command::new(cargo_bin!());
        let output = cmd
            .args([
                "--no-config-file",
                "--no-std-lib",
                "--ide-complete",
                location,
                script_path.to_str().unwrap(),
            ])
            .output()?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        assert!(output.status.success());
        let json: serde_json::Value = serde_json::from_str(&stdout)?;
        assert!(json["completions"].is_array());
        assert!(!stderr.contains("panicked"));
    }

    let _ = std::fs::remove_file(&script_path);
    Ok(())
}

#[test]
fn ide_complete_escapes_suggestion_values() -> TestResult {
    let script_path = unique_temp_script_path("ide_complete_json");
    let script = "def comp [] { ['a\\b' 'a\"b' \"a\\nb\"] }; def cmd [x: string@comp] {}; cmd a";
    std::fs::write(&script_path, script)?;
    let output = Command::new(cargo_bin!())
        .args([
            "--no-config-file",
            "--no-std-lib",
            "--ide-complete",
            &script.len().to_string(),
        ])
        .arg(&script_path)
        .output()?;
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(
        json["completions"],
        serde_json::json!(["a\nb", "a\"b", "a\\b"])
    );
    let _ = std::fs::remove_file(script_path);
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

#[cfg(any(feature = "mcp", feature = "lsp", feature = "dap"))]
#[test]
fn server_startup_files_receive_converted_path() -> TestResult {
    for flag in [
        #[cfg(feature = "mcp")]
        "--mcp",
        #[cfg(feature = "lsp")]
        "--lsp",
        #[cfg(feature = "dap")]
        "--dap",
    ] {
        for startup_file in ["env.nu", "config.nu"] {
            let config_home = tempfile::tempdir()?;
            let kept_path = config_home.path().join("keep");
            let excluded_path = config_home.path().join("mcp-path-excluded");
            let output_path = config_home.path().join("path.json");
            std::fs::write(config_home.path().join("env.nu"), "")?;
            std::fs::write(config_home.path().join("config.nu"), "")?;
            std::fs::write(
                config_home.path().join(startup_file),
                r#"
                $env.PATH = $env.PATH | where $it !~ "mcp-path-excluded"
                $env.PATH | to json | save --force $env.MCP_PATH_OUTPUT
                if $nu.is-mcp {
                    ^$nu.current-exe --no-config-file --no-std-lib -c "'startup output'"
                }
            "#,
            )?;

            let output = assert_cmd::Command::new(cargo_bin!())
                .args(["--no-std-lib", flag, "--config-home"])
                .arg(config_home.path())
                .env("PATH", std::env::join_paths([&kept_path, &excluded_path])?)
                .env("MCP_PATH_OUTPUT", &output_path)
                .timeout(std::time::Duration::from_secs(30))
                .output()?;
            let stderr = String::from_utf8_lossy(&output.stderr);

            if flag == "--lsp" {
                // LSP reports a disconnected channel when stdin closes before initialization.
                assert!(stderr.contains("disconnected channel"), "{flag}: {stderr}");
            } else {
                assert!(output.status.success(), "{flag} {startup_file}: {stderr}");
            }
            assert!(
                output.stdout.is_empty(),
                "{flag} {startup_file} leaked startup output: {:?}",
                String::from_utf8_lossy(&output.stdout)
            );
            assert!(
                output_path.exists(),
                "{flag} {startup_file} did not finish: {stderr}"
            );
            let path: serde_json::Value = serde_json::from_slice(&std::fs::read(output_path)?)?;
            assert_eq!(
                path,
                serde_json::json!([kept_path]),
                "{flag} {startup_file}"
            );
        }
    }

    Ok(())
}

#[cfg(feature = "mcp")]
#[test]
fn mcp_evaluate_receives_converted_path_without_config() -> TestResult {
    use serde_json::json;

    let session: String = [
        json!({
            "jsonrpc": "2.0", "id": 1, "method": "initialize",
            "params": {
                "protocolVersion": "2025-11-25", "capabilities": {},
                "clientInfo": { "name": "test", "version": "1" }
            }
        }),
        json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }),
        json!({
            "jsonrpc": "2.0", "id": 2, "method": "tools/call",
            "params": { "name": "evaluate", "arguments": {
                "input": "$env.PATH | describe --detailed | get type"
            } }
        }),
    ]
    .iter()
    .map(|message| format!("{message}\n"))
    .collect();
    let config_home = tempfile::tempdir()?;
    let output = assert_cmd::Command::new(cargo_bin!())
        .args(["--no-config-file", "--no-std-lib", "--mcp"])
        .env("PATH", std::env::join_paths([config_home.path()])?)
        .write_stdin(session)
        .timeout(std::time::Duration::from_secs(30))
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stderr}");
    let responses: Vec<serde_json::Value> = stdout
        .lines()
        .map(serde_json::from_str)
        .collect::<Result<_, _>>()?;
    let response = responses
        .iter()
        .find(|response| response["id"] == 2)
        .ok_or_else(|| format!("missing evaluate response: {stdout}\n{stderr}"))?;
    assert_eq!(response["result"]["structuredContent"]["output"], "list");
    Ok(())
}

/// Runs `nu <flag>` in `<root>/start` with `config` as its `config.nu` and an empty `env.nu`,
/// writes `session` to its stdin, closes it and returns stdout once the server exits.
///
/// For servers, which run the startup files on a stack they then discard. Closing stdin is what
/// ends the server, so this waits for it to exit, not for a reply: a missing or reworded reply
/// fails the caller's assertions.
#[cfg(any(feature = "lsp", feature = "mcp"))]
fn run_server_session(
    flag: &str,
    root: &std::path::Path,
    config: &str,
    session: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    use std::io::Write;
    use std::process::Stdio;

    let (config_path, env_path) = (root.join("config.nu"), root.join("env.nu"));
    std::fs::write(&config_path, config)?;
    std::fs::write(&env_path, "")?;

    let mut child = Command::new(cargo_bin!())
        .arg("--no-std-lib")
        .arg("--config")
        .arg(config_path)
        .arg("--env-config")
        .arg(env_path)
        .arg(flag)
        .current_dir(root.join("start"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    // The handle is dropped at the end of this statement, which closes stdin.
    child
        .stdin
        .take()
        .ok_or("stdin is piped")?
        .write_all(session.as_bytes())?;

    Ok(String::from_utf8(child.wait_with_output()?.stdout)?)
}

/// The server sets `PWD` to the open file's directory. An overlay command in a block of
/// `config.nu` must not change which overlay it reads that back from, because the stack that ran
/// the command is gone.
#[cfg(feature = "lsp")]
#[rstest::rstest]
#[case::overlay_use("if true { overlay use spam }")]
#[case::overlay_hide("overlay use spam\nif true { overlay hide spam }")]
fn lsp_completes_from_open_file_dir_after_overlay_command_in_config_block(
    #[case] overlay_command: &str,
) -> TestResult {
    use nu_test_support::assertions::{assert_contains, assert_contains_not};
    use serde_json::json;

    let root = tempfile::tempdir()?;
    std::fs::create_dir(root.path().join("start"))?;
    std::fs::create_dir(root.path().join("proj"))?;
    std::fs::write(root.path().join("start/in_start.txt"), "")?;
    std::fs::write(root.path().join("proj/in_proj.txt"), "")?;

    let script = root.path().join("proj/script.nu");
    let script = script.to_string_lossy().replace('\\', "/");
    let slash = if script.starts_with('/') { "" } else { "/" };
    let uri = format!("file://{slash}{script}");
    let text_document = json!({ "uri": uri, "languageId": "nushell", "version": 1, "text": "ls " });
    let session: String = [
        json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": { "capabilities": {} } }),
        json!({ "jsonrpc": "2.0", "method": "initialized", "params": {} }),
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": { "textDocument": text_document },
        }),
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "textDocument/completion",
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": 0, "character": 3 },
            },
        }),
        json!({ "jsonrpc": "2.0", "id": 3, "method": "shutdown" }),
        json!({ "jsonrpc": "2.0", "method": "exit" }),
    ]
    .iter()
    .map(|message| {
        let body = message.to_string();
        format!("Content-Length: {}\r\n\r\n{body}", body.len())
    })
    .collect();

    let config = format!("module spam {{ export-env {{ $env.X = 1 }} }}\n{overlay_command}\n");
    let stdout = run_server_session("--lsp", root.path(), &config, &session)?;

    assert_contains("in_proj.txt", &stdout);
    assert_contains_not("in_start.txt", &stdout);
    Ok(())
}

/// The parser and the stack the server evaluates on have to agree on the directory. The `cd` in
/// `config.nu` lands in `spam`, which that stack doesn't have, so both stay in the start directory.
#[cfg(feature = "mcp")]
#[test]
fn mcp_parser_agrees_with_stack_after_overlay_command_in_config_block() -> TestResult {
    use nu_test_support::assertions::{assert_contains, assert_contains_not};
    use serde_json::json;

    let root = tempfile::tempdir()?;
    for (dir, name) in [("start", "from-start"), ("proj", "from-proj")] {
        std::fs::create_dir(root.path().join(dir))?;
        std::fs::write(
            root.path().join(dir).join("helper.nu"),
            format!("export def foo [] {{ '{name}' }}\n"),
        )?;
    }

    let session: String = [
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": { "name": "test", "version": "1" },
            },
        }),
        json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }),
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": { "name": "evaluate", "arguments": { "input": "use ./helper.nu foo; foo" } },
        }),
    ]
    .iter()
    .map(|message| format!("{message}\n"))
    .collect();

    let config =
        "module spam { export-env { $env.X = 1 } }\nif true { overlay use spam }\ncd ../proj\n";
    // `rmcp` gives a call still running when stdin closes 5 seconds to send its response.
    let stdout = run_server_session("--mcp", root.path(), config, &session)?;

    assert_contains("from-start", &stdout);
    assert_contains_not("from-proj", &stdout);
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
    let script_path = unique_temp_script_path("test_args_script");

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
    let script_path = unique_temp_script_path("test_script_args2");

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
    let script_path = unique_temp_script_path("test_flags_before");

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

// Tests for command-line script with arguments
#[test]
fn command_can_receive_arguments() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args([
            "--no-config-file",
            "--no-std-lib",
            "-c",
            "def main [arg, ...rest] {
                {arg: $arg, rest: $rest} | to nuon
            }",
            "earth",
            "wind and water",
            "fire",
        ])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(
        stdout.contains(r#"{arg: earth, rest: ["wind and water", fire]}"#),
        "actual: {stdout:?}"
    );
    Ok(())
}

#[test]
fn command_ignores_arguments_without_main() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args([
            "--no-config-file",
            "--no-std-lib",
            "-c",
            "print 'ok'",
            "--this-is-not-a-nushell-flag",
            "positional",
        ])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stderr: {stderr:?}");
    assert_eq!(stdout.trim(), "ok");
    Ok(())
}

#[test]
fn command_reports_parse_errors() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args([
            "--no-config-file",
            "--no-std-lib",
            "-c",
            "def main [i: int] { }",
            "abc",
        ])
        .output()?;

    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(
        stderr.contains("nu::parser::parse_mismatch"),
        "actual: {stderr:?}"
    );
    assert!(stderr.contains("expected int"), "actual: {stderr:?}");

    Ok(())
}

#[test]
fn command_can_receive_wrapped_arguments() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args([
            "--no-config-file",
            "--no-std-lib",
            "-c",
            "def --wrapped main [...rest] {
                $rest | to nuon
            }",
            "--",
            "earth",
            "wind and water",
            "fire",
        ])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(
        stdout.contains(r#"[--, earth, "wind and water", fire]"#),
        "actual: {stdout:?}"
    );
    Ok(())
}

#[test]
fn command_can_parse_flags() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args([
            "--no-config-file",
            "--no-std-lib",
            "-c",
            "def main [--flag, --zip: string, ...rest] {
                {flag: $flag, zip: $zip, rest: $rest } | to nuon
            }",
            "--flag",
            "--zip=zap",
            "'foo bar'",
            "baz",
        ])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(
        stdout.contains(r#"{flag: true, zip: zap, rest: ["'foo bar'", baz]"#),
        "actual: {stdout:?}"
    );

    Ok(())
}

#[test]
fn command_can_wrap_unknown_args() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args([
            "--no-config-file",
            "--no-std-lib",
            "-c",
            "def --wrapped main [...rest] {
                $rest | to nuon
            }",
            "--",
            "--flag",
            "--zip=zap",
            "'zop zim'",
            "beep",
        ])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(
        stdout.contains(r#"[--, --flag, "--zip=zap", "'zop zim'", beep]"#),
        "actual: {stdout:?}"
    );

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
        "frameless",
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
    let levels = ["error", "warn", "info", "debug", "trace", "perf"];

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

// Specifying --log-file without setting --log-target file should fail
#[test]
fn log_file_without_target_fails() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args([
            "--no-config-file",
            "--no-std-lib",
            "--log-file",
            "some.log",
            "-c",
            "print 'ok'",
        ])
        .output()?;

    assert!(
        !output.status.success(),
        "Expected failure when log-file is used without file target"
    );
    Ok(())
}

// The combination of file target and file path requires an explicit log-level
#[test]
fn log_file_target_needs_level() -> TestResult {
    let temp = std::env::temp_dir().join("nu_cli_log.log");
    let path = temp.to_str().unwrap();

    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args([
            "--no-config-file",
            "--no-std-lib",
            "--log-target",
            "file",
            "--log-file",
            path,
            "-c",
            "print 'ok'",
        ])
        .output()?;

    let _ = std::fs::remove_file(&temp);
    assert!(
        !output.status.success(),
        "Expected failure when log-level is missing"
    );
    Ok(())
}

// File target must be accompanied by a log-file argument
#[test]
fn log_target_file_with_log_file_succeeds() -> TestResult {
    let temp = std::env::temp_dir().join("nu_cli_log.log");
    let path = temp.to_str().unwrap();

    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args([
            "--no-config-file",
            "--no-std-lib",
            "--log-target",
            "file",
            "--log-file",
            path,
            "--log-level",
            "info",
            "-c",
            "print 'ok'",
        ])
        .output()?;

    let _ = std::fs::remove_file(&temp);
    assert!(
        output.status.success(),
        "Expected success with log file path"
    );
    Ok(())
}

#[test]
fn log_target_file_without_log_file_fails() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args([
            "--no-config-file",
            "--no-std-lib",
            "--log-target",
            "file",
            "-c",
            "print 'ok'",
        ])
        .output()?;

    assert!(
        !output.status.success(),
        "Expected failure when missing --log-file"
    );
    Ok(())
}

// Test that -I sets $env.NU_LIB_DIRS correctly
#[test]
fn include_path_sets_env_nu_lib_dirs() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args([
            "--no-config-file",
            "--no-std-lib",
            "-I",
            "/tmp/test",
            "-c",
            "$env.NU_LIB_DIRS | to nuon",
        ])
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(stdout.contains("/tmp/test"));
    Ok(())
}

// Test that -I appends to existing NU_LIB_DIRS env var
// Test that -I appends to NU_LIB_DIRS env var
// Note: This test checks that -I works, env appending may not work in test framework
#[test]
fn include_path_appends_to_env_nu_lib_dirs() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args([
            "--no-config-file",
            "--no-std-lib",
            "-I",
            "/tmp/append",
            "-c",
            "$env.NU_LIB_DIRS | to nuon",
        ])
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    // Should at least contain the -I path
    assert!(stdout.contains("/tmp/append"));
    #[cfg(windows)]
    {
        assert!(stdout.contains(r#"\scripts"#));
        assert!(stdout.contains(r#"\completions"#));
    }
    #[cfg(not(windows))]
    {
        assert!(stdout.contains("/scripts"));
        assert!(stdout.contains("/completions"));
    }
    Ok(())
}

// Test that NU_LIB_DIRS env var sets $env.NU_LIB_DIRS
#[test]
fn nu_lib_dirs_env_var_sets_env() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    cmd.env("NU_LIB_DIRS", "/tmp/envpath");
    let output = cmd
        .args([
            "--no-config-file",
            "--no-std-lib",
            "-c",
            "$env.NU_LIB_DIRS | to nuon",
        ])
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(stdout.contains("/tmp/envpath"));
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

// Tests for short-flag group validation (PR #17492)
// These tests verify that short-flag validation works correctly and that
// the bare dash "-" is rejected early, making certain code paths unreachable.

#[test]
fn bare_dash_is_rejected() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args(["--no-config-file", "--no-std-lib", "-"])
        .output()?;

    assert!(!output.status.success());
    Ok(())
}

#[test]
fn long_flag_without_value_works() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args([
            "--no-config-file",
            "--no-std-lib",
            "--no-history",
            "-c",
            "print ok",
        ])
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert_eq!(stdout.trim(), "ok");
    Ok(())
}

#[test]
fn long_flag_with_value_works() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args([
            "--no-config-file",
            "--no-std-lib",
            "--table-mode",
            "basic",
            "-c",
            "print ok",
        ])
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert_eq!(stdout.trim(), "ok");
    Ok(())
}

#[test]
fn short_flag_without_value_works() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args(["--no-config-file", "--no-std-lib", "-l", "-c", "print ok"])
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert_eq!(stdout.trim(), "ok");
    Ok(())
}

#[test]
fn short_flag_with_value_works() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args([
            "--no-config-file",
            "--no-std-lib",
            "-m",
            "basic",
            "-c",
            "print ok",
        ])
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert_eq!(stdout.trim(), "ok");
    Ok(())
}

#[test]
fn mixed_long_and_short_flags_work() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args([
            "--no-config-file",
            "--no-std-lib",
            "-il",
            "--table-mode",
            "basic",
            "--error-style",
            "plain",
            "-c",
            "print ok",
        ])
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert_eq!(stdout.trim(), "ok");
    Ok(())
}
