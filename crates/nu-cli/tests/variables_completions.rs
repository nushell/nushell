pub mod support;

use nu_cli::NuCompleter;
use reedline::Completer;
use support::{match_suggestions, new_engine};

#[test]
fn variables_completions() {
    // Create a new engine
    let (dir, _, mut engine, mut stack) = new_engine();

    // Add record value as example
    let record = "let actor = { name: 'Tom Hardy', age: 44 }";
    assert!(support::merge_input(record.as_bytes(), &mut engine, &mut stack, dir).is_ok());

    // Instatiate a new completer
    let mut completer = NuCompleter::new(std::sync::Arc::new(engine), stack);

    // Test completions for $nu
    let suggestions = completer.complete("$nu.".into(), 4);

    assert_eq!(8, suggestions.len());

    let expected: Vec<String> = vec![
        "config-path".into(),
        "env-path".into(),
        "history-path".into(),
        "home-path".into(),
        "os-info".into(),
        "pid".into(),
        "scope".into(),
        "temp-path".into(),
    ];

    // Match results
    match_suggestions(expected, suggestions);

    // Test completions for $nu.h (filter)
    let suggestions = completer.complete("$nu.h".into(), 5);

    assert_eq!(2, suggestions.len());

    let expected: Vec<String> = vec!["history-path".into(), "home-path".into()];

    // Match results
    match_suggestions(expected, suggestions);

    // Test completions for custom var
    let suggestions = completer.complete("$actor.".into(), 7);

    assert_eq!(2, suggestions.len());

    let expected: Vec<String> = vec!["age".into(), "name".into()];

    // Match results
    match_suggestions(expected, suggestions);

    // Test completions for custom var (filtering)
    let suggestions = completer.complete("$actor.n".into(), 7);

    assert_eq!(1, suggestions.len());

    let expected: Vec<String> = vec!["name".into()];

    // Match results
    match_suggestions(expected, suggestions);

    // Test completions for $env
    let suggestions = completer.complete("$env.".into(), 5);

    assert_eq!(2, suggestions.len());

    let expected: Vec<String> = vec!["PWD".into(), "TEST".into()];

    // Match results
    match_suggestions(expected, suggestions);

    // Test completions for $env
    let suggestions = completer.complete("$env.T".into(), 5);

    assert_eq!(1, suggestions.len());

    let expected: Vec<String> = vec!["TEST".into()];

    // Match results
    match_suggestions(expected, suggestions);
}
