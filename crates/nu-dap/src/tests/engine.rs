//! Unit tests for [`crate::engine`].

use crate::engine::{Target, parse_script};
use nu_protocol::engine::EngineState;

/// A target that never touches the filesystem: `Target::resolve` reads the
/// script from disk, but parsing only needs its bytes.
fn target(contents: &str) -> Target {
    Target {
        program: std::path::PathBuf::from("script.nu"),
        contents: contents.as_bytes().to_vec(),
        cwd: ".".into(),
    }
}

/// The baseline for the test below: a script with neither kind of error.
/// Plain arithmetic, because a bare `EngineState` carries no declarations —
/// not even `let`, which comes from nu-cmd-lang.
#[test]
fn valid_script_parses() {
    let mut engine_state = EngineState::new();
    parse_script(&mut engine_state, &target("1 + 1\n")).expect("parses");
}

/// Parse errors and compile errors are separate channels, and only the parse
/// side is obvious. `$env.PWD = ...` parses fine and fails to compile; if the
/// compile error were not reported here the block would reach `eval_block`
/// with no `ir_block`, and the user would see nushell's internal "block is
/// missing compiled representation" instead of the real problem.
#[test]
fn compile_error_is_reported_as_a_launch_failure() {
    let mut engine_state = EngineState::new();
    let err = parse_script(&mut engine_state, &target("$env.PWD = \"/tmp\"\n"))
        .expect_err("compile error");

    assert!(
        err.starts_with("compile error:"),
        "should be reported as a compile error, not a parse error: {err}"
    );

    assert!(
        err.contains("PWD cannot be set manually"),
        "should carry nushell's own message: {err}"
    );
}
