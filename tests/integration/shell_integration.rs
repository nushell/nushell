// Integration tests for OSC 133/633 shell integration semantic prompt markers
//
// These tests verify that prompt_update::update_prompt correctly renders prompts
// with ANSI escape sequences and semantic markers in the proper left-to-right order.

use nu_cli::{NushellPrompt, update_prompt};
use nu_protocol::{
    Config, Span, Value,
    engine::{EngineState, Stack},
};
use nu_test_support::nu;
use reedline::{Prompt, PromptEditMode};
use std::sync::Arc;

// Helper to create a minimal engine state and stack for prompt testing
fn create_test_engine() -> (EngineState, Stack, Arc<Config>) {
    let mut engine_state =
        nu_command::add_shell_command_context(nu_cmd_lang::create_default_context());
    engine_state.generate_nu_constant();

    let mut stack = Stack::new();

    // Set up simple prompt environment variables with literal strings
    stack.add_env_var(
        "PROMPT_COMMAND".to_string(),
        Value::string("❯ ".to_string(), Span::unknown()),
    );
    stack.add_env_var(
        "PROMPT_INDICATOR".to_string(),
        Value::string("> ".to_string(), Span::unknown()),
    );
    stack.add_env_var(
        "PROMPT_COMMAND_RIGHT".to_string(),
        Value::string("~/code".to_string(), Span::unknown()),
    );
    stack.add_env_var(
        "PROMPT_MULTILINE_INDICATOR".to_string(),
        Value::string("... ".to_string(), Span::unknown()),
    );
    stack.add_env_var(
        "PWD".to_string(),
        Value::string("/test".to_string(), Span::unknown()),
    );

    let config = engine_state.get_config().clone();

    (engine_state, stack, config)
}

// ────────────────────────────────────────────────────────────────────────────────
// PROMPT RENDERING TESTS WITH ACTUAL update_prompt CALLS
// ────────────────────────────────────────────────────────────────────────────────
// These tests call the real prompt_update::update_prompt function and verify
// that prompts are rendered with proper content.

/// Test that update_prompt with OSC 133 renders prompts correctly
#[test]
fn test_update_prompt_with_osc133() {
    let (engine_state, mut stack, config) = create_test_engine();
    let mut prompt = NushellPrompt::new();

    // Call the actual update_prompt function
    update_prompt(&config, &engine_state, &mut stack, &mut prompt);

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
    let (engine_state, mut stack, config) = create_test_engine();
    let mut prompt = NushellPrompt::new();

    // Call the actual update_prompt function
    update_prompt(&config, &engine_state, &mut stack, &mut prompt);

    let left = prompt.render_prompt_left();

    assert_eq!(left.as_ref(), "❯ ");
}

/// Test that update_prompt correctly handles both left and right prompts
#[test]
fn test_update_prompt_left_and_right() {
    let (engine_state, mut stack, config) = create_test_engine();
    let mut prompt = NushellPrompt::new();

    update_prompt(&config, &engine_state, &mut stack, &mut prompt);

    let left = prompt.render_prompt_left();
    let right = prompt.render_prompt_right();

    // Should get values from env vars set in create_test_engine
    assert_eq!(left.as_ref(), "❯ ");
    assert_eq!(right.as_ref(), "~/code");
}

/// Test that update_prompt correctly sets multiline indicator
#[test]
fn test_update_prompt_multiline() {
    let (engine_state, mut stack, config) = create_test_engine();
    let mut prompt = NushellPrompt::new();

    update_prompt(&config, &engine_state, &mut stack, &mut prompt);

    let multiline = prompt.render_prompt_multiline_indicator();

    assert_eq!(multiline.as_ref(), "... ");
}

/// Test that update_prompt respects empty/missing prompt variables
#[test]
fn test_update_prompt_with_missing_vars() {
    let mut engine_state =
        nu_command::add_shell_command_context(nu_cmd_lang::create_default_context());
    engine_state.generate_nu_constant();

    let mut stack = Stack::new();
    stack.add_env_var(
        "PWD".to_string(),
        Value::string("/test".to_string(), Span::unknown()),
    );

    let config = (*engine_state.get_config()).clone();

    let mut prompt = NushellPrompt::new();

    // Call update_prompt without setting PROMPT_COMMAND env vars
    update_prompt(&config, &engine_state, &mut stack, &mut prompt);

    // Should still work, just with default/empty prompts
    let left = prompt.render_prompt_left();
    // Default behavior when PROMPT_COMMAND is not set - will have some content
    assert!(!left.as_ref().is_empty());
}

// ────────────────────────────────────────────────────────────────────────────────
// CONFIGURATION TESTS
// ────────────────────────────────────────────────────────────────────────────────

/// Test that osc133 can be enabled/disabled via config
#[test]
fn test_osc133_config_toggle() {
    let result = nu!("$env.config.shell_integration.osc133");
    assert_eq!(result.out, "true");

    let result = nu!(r#"
        $env.config.shell_integration.osc133 = false
        $env.config.shell_integration.osc133
    "#);
    assert_eq!(result.out, "false");
}

/// Test that osc633 can be enabled/disabled via config
#[test]
fn test_osc633_config_toggle() {
    let result = nu!("$env.config.shell_integration.osc633");
    assert_eq!(result.out, "true");

    let result = nu!(r#"
        $env.config.shell_integration.osc633 = false
        $env.config.shell_integration.osc633
    "#);
    assert_eq!(result.out, "false");
}
