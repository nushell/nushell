pub mod support;

use nu_cli::NuCompleter;
use reedline::Completer;
use support::{match_suggestions, new_engine};

#[test]
fn variables_completions() {
    // Create a new engine
    let (dir, _, mut engine, mut stack) = new_engine();

    // Add record value as example
    let record = r#"def animals [] { ["cat", "dog", "eel" ] }
    def my-command [animal: string@animals] { print $animal }"#;
    assert!(support::merge_input(record.as_bytes(), &mut engine, &mut stack, dir).is_ok());

    // Instatiate a new completer
    let mut completer = NuCompleter::new(std::sync::Arc::new(engine), stack);

    // Test completions for $nu
    let suggestions = completer.complete("my-command ".into(), 11);

    assert_eq!(3, suggestions.len());

    let expected: Vec<String> = vec!["cat".into(), "dog".into(), "eel".into()];

    // Match results
    match_suggestions(expected, suggestions);
}
