//! The history session id reported while the startup config is evaluated (#18321).

use nu_test_support::prelude::*;
use std::fs;
#[cfg(not(windows))]
use std::process::{Command, Stdio};

/// Regression test for #18321: the history session id is created when the
/// REPL starts, before the startup config is evaluated, so `history session`
/// must report the real (nonzero) id during initialization.
///
/// `run_repl` evaluates the config before `evaluate_repl` checks for a TTY,
/// so driving the binary with a closed stdin prints the id and then fails
/// with the "STDIN is not a TTY" error; only stdout matters here.
#[cfg(not(windows))]
#[test]
#[deps(NU)]
fn history_session_id_reported_during_startup_config() -> Result {
    Playground::setup("history_session_startup", |_, playground| {
        let xdg = playground.cwd().join("xdg");
        fs::create_dir_all(&xdg)?;

        let config = playground.cwd().join("config.nu");
        fs::write(
            &config,
            "$env.config.history.isolation = true\nprint (history session)\n",
        )?;

        let output = Command::new(NU.path())
            .current_dir(playground.cwd())
            .arg("--config")
            .arg(&config)
            .env("XDG_CONFIG_HOME", &xdg)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let session_id: i64 = stdout
            .lines()
            .last()
            .unwrap_or("")
            .trim()
            .parse()
            .expect("history session should print the session id");
        assert_ne!(
            session_id, 0,
            "history session must report the real id during startup, stdout={stdout}"
        );

        Ok(())
    })
}
