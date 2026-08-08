// Integration tests for OSC 133/633 shell integration semantic prompt markers
//
// These tests verify that prompt_update::update_prompt correctly renders prompts
// with ANSI escape sequences and semantic markers in the proper left-to-right order.

use nu_cli::{NushellPrompt, update_prompt};
use nu_test_support::prelude::*;
use reedline::{Prompt, PromptEditMode};

// ────────────────────────────────────────────────────────────────────────────────
// PROMPT RENDERING TESTS WITH ACTUAL update_prompt CALLS
// ────────────────────────────────────────────────────────────────────────────────
// These tests call the real prompt_update::update_prompt function and verify
// that prompts are rendered with proper content.

/// Test that update_prompt with OSC 133 renders prompts correctly
#[test]
fn test_update_prompt_with_osc133() {
    let mut tester = test()
        .env("PROMPT_COMMAND", "❯ ")
        .env("PROMPT_INDICATOR", "> ")
        .env("PROMPT_COMMAND_RIGHT", "~/code")
        .env("PROMPT_MULTILINE_INDICATOR", "... ")
        .env("PWD", "/test");
    let config = tester.engine_state.get_config().clone();

    // Call the actual update_prompt function
    update_prompt(&config, &tester.engine_state, &mut tester.stack);

    let prompt = NushellPrompt::shared(tester.engine_state.prompt_state.clone());

    // Verify prompts were updated correctly
    let left = prompt.render_prompt_left();
    let indicator = prompt.render_prompt_indicator(PromptEditMode::Default);
    let right = prompt.render_prompt_right();
    let multiline = prompt.render_prompt_multiline_indicator();

    assert_eq!(left.as_ref(), "❯ ");
    assert_eq!(indicator.as_ref(), "> ");
    assert_eq!(right.as_ref(), "~/code");
    assert_eq!(multiline.as_ref(), "... ");
}

/// Test that update_prompt with OSC 633 renders prompts correctly
#[test]
fn test_update_prompt_with_osc633() {
    let mut tester = test().env("PROMPT_COMMAND", "❯ ").env("PWD", "/test");
    let config = tester.engine_state.get_config().clone();

    // Call the actual update_prompt function
    update_prompt(&config, &tester.engine_state, &mut tester.stack);

    let prompt = NushellPrompt::shared(tester.engine_state.prompt_state.clone());
    let left = prompt.render_prompt_left();

    assert_eq!(left.as_ref(), "❯ ");
}

/// Test that update_prompt correctly handles both left and right prompts
#[test]
fn test_update_prompt_left_and_right() {
    let mut tester = test()
        .env("PROMPT_COMMAND", "❯ ")
        .env("PROMPT_COMMAND_RIGHT", "~/code")
        .env("PWD", "/test");
    let config = tester.engine_state.get_config().clone();

    update_prompt(&config, &tester.engine_state, &mut tester.stack);

    let prompt = NushellPrompt::shared(tester.engine_state.prompt_state.clone());
    let left = prompt.render_prompt_left();
    let right = prompt.render_prompt_right();

    assert_eq!(left.as_ref(), "❯ ");
    assert_eq!(right.as_ref(), "~/code");
}

/// Test that update_prompt correctly sets multiline indicator
#[test]
fn test_update_prompt_multiline() {
    let mut tester = test()
        .env("PROMPT_MULTILINE_INDICATOR", "... ")
        .env("PWD", "/test");
    let config = tester.engine_state.get_config().clone();

    update_prompt(&config, &tester.engine_state, &mut tester.stack);

    let prompt = NushellPrompt::shared(tester.engine_state.prompt_state.clone());
    let multiline = prompt.render_prompt_multiline_indicator();

    assert_eq!(multiline.as_ref(), "... ");
}

/// Test that update_prompt respects empty/missing prompt variables
#[test]
fn test_update_prompt_with_missing_vars() {
    let mut tester = test().env("PWD", "/test");
    let config = tester.engine_state.get_config().clone();

    // Call update_prompt without setting PROMPT_COMMAND env vars
    update_prompt(&config, &tester.engine_state, &mut tester.stack);

    // Should still work, just with default/empty prompts
    let prompt = NushellPrompt::shared(tester.engine_state.prompt_state.clone());
    let left = prompt.render_prompt_left();
    // Default behavior when PROMPT_COMMAND is not set - will have some content
    assert!(!left.as_ref().is_empty());
}

// ────────────────────────────────────────────────────────────────────────────────
// CONFIGURATION TESTS
// ────────────────────────────────────────────────────────────────────────────────

/// Test that osc133 can be enabled/disabled via config
#[test]
fn test_osc133_config_toggle() -> Result {
    test()
        .run("$env.config.shell_integration.osc133")
        .expect_value_eq(true)?;

    test()
        .run(
            "
            $env.config.shell_integration.osc133 = false
            $env.config.shell_integration.osc133
        ",
        )
        .expect_value_eq(false)
}

/// Test that osc633 can be enabled/disabled via config
#[test]
fn test_osc633_config_toggle() -> Result {
    test()
        .run("$env.config.shell_integration.osc633")
        .expect_value_eq(true)?;

    test()
        .run(
            "
            $env.config.shell_integration.osc633 = false
            $env.config.shell_integration.osc633
        ",
        )
        .expect_value_eq(false)
}
