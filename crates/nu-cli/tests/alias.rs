pub mod support;

use nu_cli::NuCompleter;
use reedline::Completer;
use support::{match_suggestions, new_engine};

#[test]
fn alias_of_command_and_flags() {
    let (dir, _, mut engine, mut stack) = new_engine();

    // Create an alias
    let alias = r#"alias ll = ls -l"#;
    assert!(support::merge_input(alias.as_bytes(), &mut engine, &mut stack, dir.clone()).is_ok());

    let mut completer = NuCompleter::new(std::sync::Arc::new(engine), stack);

    let suggestions = completer.complete("ll t", 4);
    #[cfg(windows)]
    let expected_paths: Vec<String> = vec!["test_a\\".to_string(), "test_b\\".to_string()];
    #[cfg(not(windows))]
    let expected_paths: Vec<String> = vec!["test_a/".to_string(), "test_b/".to_string()];

    match_suggestions(expected_paths, suggestions)
}

#[test]
fn alias_of_basic_command() {
    let (dir, _, mut engine, mut stack) = new_engine();

    // Create an alias
    let alias = r#"alias ll = ls "#;
    assert!(support::merge_input(alias.as_bytes(), &mut engine, &mut stack, dir.clone()).is_ok());

    let mut completer = NuCompleter::new(std::sync::Arc::new(engine), stack);

    let suggestions = completer.complete("ll t", 4);
    #[cfg(windows)]
    let expected_paths: Vec<String> = vec!["test_a\\".to_string(), "test_b\\".to_string()];
    #[cfg(not(windows))]
    let expected_paths: Vec<String> = vec!["test_a/".to_string(), "test_b/".to_string()];

    match_suggestions(expected_paths, suggestions)
}

#[test]
fn alias_of_another_alias() {
    let (dir, _, mut engine, mut stack) = new_engine();

    // Create an alias
    let alias = r#"alias ll = ls -la"#;
    assert!(support::merge_input(alias.as_bytes(), &mut engine, &mut stack, dir.clone()).is_ok());
    // Create the second alias
    let alias = r#"alias lf = ll -f"#;
    assert!(support::merge_input(alias.as_bytes(), &mut engine, &mut stack, dir.clone()).is_ok());

    let mut completer = NuCompleter::new(std::sync::Arc::new(engine), stack);

    let suggestions = completer.complete("lf t", 4);
    #[cfg(windows)]
    let expected_paths: Vec<String> = vec!["test_a\\".to_string(), "test_b\\".to_string()];
    #[cfg(not(windows))]
    let expected_paths: Vec<String> = vec!["test_a/".to_string(), "test_b/".to_string()];

    match_suggestions(expected_paths, suggestions)
}
