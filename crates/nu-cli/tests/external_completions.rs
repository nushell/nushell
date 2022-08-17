pub mod support;

use nu_cli::NuCompleter;
use nu_parser::parse;
use nu_protocol::engine::StateWorkingSet;
use reedline::{Completer, Suggestion};
use support::new_engine;

#[test]
fn external_completer_trailing_space() {
    let block = "let external_completer = {|spans| $spans}";
    let input = "gh alias ".to_string();

    let suggestions = run_completion(&block, &input);
    assert_eq!(3, suggestions.len());
    assert_eq!("gh", suggestions.get(0).unwrap().value);
    assert_eq!("alias", suggestions.get(1).unwrap().value);
    assert_eq!("", suggestions.get(2).unwrap().value);
}

#[test]
fn external_completer_no_trailing_space() {
    let block = "let external_completer = {|spans| $spans}";
    let input = "gh alias".to_string();

    let suggestions = run_completion(&block, &input);
    assert_eq!(2, suggestions.len());
    assert_eq!("gh", suggestions.get(0).unwrap().value);
    assert_eq!("alias", suggestions.get(1).unwrap().value);
}

#[test]
fn external_completer_pass_flags() {
    let block = "let external_completer = {|spans| $spans}";
    let input = "gh api --".to_string();

    let suggestions = run_completion(&block, &input);
    assert_eq!(3, suggestions.len());
    assert_eq!("gh", suggestions.get(0).unwrap().value);
    assert_eq!("alias", suggestions.get(1).unwrap().value);
    assert_eq!("--", suggestions.get(2).unwrap().value);
}

fn run_completion(block: &str, input: &str) -> Vec<Suggestion> {
    // Create a new engine
    let (dir, _, mut engine_state, mut stack) = new_engine();
    let (_, delta) = {
        let mut working_set = StateWorkingSet::new(&engine_state);
        let (block, err) = parse(&mut working_set, None, block.as_bytes(), false, &[]);
        assert!(err.is_none());

        (block, working_set.render())
    };

    assert!(engine_state.merge_delta(delta).is_ok());

    // Merge environment into the permanent state
    assert!(engine_state.merge_env(&mut stack, &dir).is_ok());

    let latest_block_id = engine_state.num_blocks() - 1;

    // Change config adding the external completer
    let mut config = engine_state.get_config().clone();
    config.external_completer = Some(latest_block_id);
    engine_state.set_config(&config);

    // Instatiate a new completer
    let mut completer = NuCompleter::new(std::sync::Arc::new(engine_state), stack);

    completer.complete(&input, input.len())
}
