pub mod support;

use nu_cli::NuCompleter;
use reedline::Completer;
use rstest::{fixture, rstest};
use support::{match_suggestions, new_engine};

#[fixture]
fn completer() -> NuCompleter {
    // Create a new engine
    let (dir, _, mut engine, mut stack) = new_engine();

    // Add record value as example
    let record = "def tst [--mod -s] {}";
    assert!(support::merge_input(record.as_bytes(), &mut engine, &mut stack, dir).is_ok());

    // Instantiate a new completer
    NuCompleter::new(std::sync::Arc::new(engine), stack)
}

#[fixture]
fn completer_strings() -> NuCompleter {
    // Create a new engine
    let (dir, _, mut engine, mut stack) = new_engine();

    // Add record value as example
    let record = r#"def animals [] { ["cat", "dog", "eel" ] }
    def my-command [animal: string@animals] { print $animal }"#;
    assert!(support::merge_input(record.as_bytes(), &mut engine, &mut stack, dir).is_ok());

    // Instantiate a new completer
    NuCompleter::new(std::sync::Arc::new(engine), stack)
}

#[rstest]
fn variables_completions_double_dash_argument(mut completer: NuCompleter) {
    let suggestions = completer.complete("tst --", 6);
    let expected: Vec<String> = vec!["--help".into(), "--mod".into()];
    // dbg!(&expected, &suggestions);
    match_suggestions(expected, suggestions);
}

#[rstest]
fn variables_completions_single_dash_argument(mut completer: NuCompleter) {
    let suggestions = completer.complete("tst -", 5);
    let expected: Vec<String> = vec!["--help".into(), "--mod".into(), "-h".into(), "-s".into()];
    match_suggestions(expected, suggestions);
}

#[rstest]
fn variables_completions_command(mut completer_strings: NuCompleter) {
    let suggestions = completer_strings.complete("my-command ", 9);
    let expected: Vec<String> = vec!["my-command".into()];
    match_suggestions(expected, suggestions);
}

#[rstest]
fn variables_completions_subcommands(mut completer_strings: NuCompleter) {
    let suggestions = completer_strings.complete("my-command ", 11);
    let expected: Vec<String> = vec!["cat".into(), "dog".into(), "eel".into()];
    match_suggestions(expected, suggestions);
}

#[rstest]
fn variables_completions_subcommands_2(mut completer_strings: NuCompleter) {
    let suggestions = completer_strings.complete("my-command ", 11);
    let expected: Vec<String> = vec!["cat".into(), "dog".into(), "eel".into()];
    match_suggestions(expected, suggestions);
}
