pub mod support;

use nu_cli::NuCompleter;
use reedline::Completer;
use support::{match_suggestions, new_engine};

#[test]
fn flag_completions() {
    // Create a new engine
    let (_, _, engine, stack) = new_engine();

    // Instatiate a new completer
    let mut completer = NuCompleter::new(std::sync::Arc::new(engine), stack);
    // Test completions for the 'ls' flags
    let suggestions = completer.complete("ls -".into(), 4);

    assert_eq!(12, suggestions.len());

    let expected: Vec<String> = vec![
        "--all".into(),
        "--du".into(),
        "--full-paths".into(),
        "--help".into(),
        "--long".into(),
        "--short-names".into(),
        "-a".into(),
        "-d".into(),
        "-f".into(),
        "-h".into(),
        "-l".into(),
        "-s".into(),
    ];

    // Match results
    match_suggestions(expected, suggestions);
}
