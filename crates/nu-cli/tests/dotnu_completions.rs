pub mod support;

use nu_cli::NuCompleter;
use reedline::Completer;
use support::new_engine;

#[test]
fn dotnu_completions() {
    // Create a new engine
    let (_, _, engine, stack) = new_engine();

    // Instatiate a new completer
    let mut completer = NuCompleter::new(std::sync::Arc::new(engine), stack);

    // Test source completion
    let completion_str = "source ".to_string();
    let suggestions = completer.complete(&completion_str, completion_str.len());

    assert_eq!(1, suggestions.len());
    assert_eq!("custom_completion.nu", suggestions.get(0).unwrap().value);

    // Test use completion
    let completion_str = "use ".to_string();
    let suggestions = completer.complete(&completion_str, completion_str.len());

    assert_eq!(1, suggestions.len());
    assert_eq!("custom_completion.nu", suggestions.get(0).unwrap().value);
}
