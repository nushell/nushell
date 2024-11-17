use super::*;
use nu_protocol::{EngineState, Stack, Value};
use nu_test_support::{nu, pipeline};
use nu_protocol::Span;

#[test]
fn test_load_allowed_schemes_from_env_without_value() {
    let engine_state = EngineState::new();
    let stack = Stack::new();

    let schemes = load_allowed_schemes_from_env(&engine_state, &stack);
    assert_eq!(schemes.len(), 2);
    assert!(schemes.contains(&"http".to_string()));
    assert!(schemes.contains(&"https".to_string()));
}

#[test]
fn test_load_allowed_schemes_from_env_with_non_string() {
    let mut engine_state = EngineState::new();
    let mut stack = Stack::new();

    // Simulate setting the environment variable to a non-string value
    let env_var = Value::Int {
        val: 42,
        span: Span::unknown(),
    };
    stack.add_env_var("ALLOWED_SCHEMES".to_string(), env_var);

    let schemes = load_allowed_schemes_from_env(&engine_state, &stack);
    assert_eq!(schemes.len(), 2);
    assert!(schemes.contains(&"http".to_string()));
    assert!(schemes.contains(&"https".to_string()));
}

#[test]
fn test_load_allowed_schemes_from_env_with_value() {
    let mut engine_state = EngineState::new();
    let mut stack = Stack::new();

    // Simulate setting the environment variable in Nushell
    let env_var = Value::String {
        val: "http,https,obsidian".to_string(),
        span: Span::unknown(),
    };
    stack.add_env_var("ALLOWED_SCHEMES".to_string(), env_var);

    let schemes = load_allowed_schemes_from_env(&engine_state, &stack);
    assert_eq!(schemes.len(), 3);
    assert!(schemes.contains(&"http".to_string()));
    assert!(schemes.contains(&"https".to_string()));
    assert!(schemes.contains(&"obsidian".to_string()));
}
